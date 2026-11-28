// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Where an algorithm searches a map for a key again with nothing between that
//! can move the key's entry (docs/adr/0161, D1): the available searches of its
//! body, as available expressions are found (Aho, Lam, Sethi, Ullman,
//! *Compilers*, 9.2.6), over a body of blocks, which is its own dominator tree.
//!
//! A **search** is the lookup `has(m, k)`, `m[k]` and `m[k].field` each make,
//! and the one `<sce:put target="m" key="k">` makes, of a map or a set `m` the
//! body holds by a name or a field of one `k`: the keys the proof that a key
//! is held reads (docs/adr/0108, D6 item 2). It is **killed** by what can move
//! the key's entry or change whether the key is held: any write of the map the
//! narrowing reads a statement as making ([`Writes`]), except a put of the map
//! at the same key, which leaves the entry in its slot or puts it where the
//! search found it absent (docs/adr/0154, D2); and any write of the key. It is
//! **available** at a statement where a search of the same map and key was made
//! before it in its block or in a block that holds it, with no kill between. A
//! loop is entered with what its body does not kill, and a search a branch or a
//! loop's body makes is not available after it.
//!
//! The plan ([`SearchPlan`]) makes a search once where it is first made and
//! is available at one more search, at least: before the statement that first
//! makes it, whose key cannot fail, and every search it reaches reads what it
//! found. A lane that keeps the result of a search lowers from the plan; one
//! that does not lowers every search as before (docs/adr/0161, D4).

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::forge::expr::{parse_to_ast, ExprKind, TypedExpr};
use crate::forge::model::AlgorithmStmt;
use crate::forge::narrowing::{subject, Writes};

/// A search of the map or set `map` for the key `key`, a name or a field of
/// one as the narrowing names a subject (`zid`, `peer.zid`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Search {
    pub(crate) map: String,
    pub(crate) key: String,
}

/// A search made once, before the statement that first makes it: the local that
/// holds what it found, and whether a put goes through it, which writes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Opened {
    pub(crate) search: Search,
    pub(crate) local: String,
    pub(crate) put_through: bool,
}

/// Which searches of a body are made once, and where each is read
/// (docs/adr/0161, D1).
///
/// A statement is named by its address in the body the plan was made from
/// ([`Self::of`]): the lowering walks that same body, which nothing moves
/// between the two.
#[derive(Debug, Default)]
pub(crate) struct SearchPlan {
    opened: HashMap<usize, Vec<Opened>>,
    through: HashMap<usize, BTreeMap<Search, String>>,
}

impl SearchPlan {
    /// The plan of `body`, whose maps and sets are the names `keyed` holds.
    pub(crate) fn of(body: &[AlgorithmStmt], keyed: &dyn Fn(&str) -> bool) -> Self {
        let mut planner = Planner {
            keyed,
            plan: Self::default(),
            made: 0,
            written: BTreeSet::new(),
        };
        planner.block(body, &BTreeMap::new());
        let written = std::mem::take(&mut planner.written);
        let mut plan = planner.plan;
        for opened in plan.opened.values_mut().flatten() {
            opened.put_through = written.contains(&opened.local);
        }
        plan
    }

    /// The searches made once before `statement`.
    pub(crate) fn opened_before(&self, statement: &AlgorithmStmt) -> &[Opened] {
        self.opened
            .get(&address(statement))
            .map_or(&[], Vec::as_slice)
    }

    /// The searches `statement`'s own expressions and its put read through a
    /// search made before, each to the local that holds what it found.
    pub(crate) fn through(&self, statement: &AlgorithmStmt) -> Option<&BTreeMap<Search, String>> {
        self.through
            .get(&address(statement))
            .filter(|through| !through.is_empty())
    }

    /// Whether no search of the body is made once.
    pub(crate) fn is_empty(&self) -> bool {
        self.opened.is_empty()
    }
}

fn address(statement: &AlgorithmStmt) -> usize {
    std::ptr::from_ref(statement) as usize
}

/// The searches available at a point, each to the local that holds what it
/// found.
type Available = BTreeMap<Search, String>;

struct Planner<'k> {
    keyed: &'k dyn Fn(&str) -> bool,
    plan: SearchPlan,
    /// How many searches the body has made once, which numbers the next.
    made: usize,
    /// The locals a put goes through.
    written: BTreeSet<String>,
}

/// What is available after a statement or a block: `None` where no path goes
/// on past it (every path returned), which a meet of two paths reads as no
/// constraint, as an unreachable point is in a dataflow.
type Flow = Option<Available>;

impl Planner<'_> {
    /// Plan `block`, entered with `entry` available; what is available after
    /// it, of what was available at its entry.
    fn block(&mut self, block: &[AlgorithmStmt], entry: &Available) -> Flow {
        let mut available = Some(entry.clone());
        for (at, statement) in block.iter().enumerate() {
            // A statement no path reaches is planned with nothing available.
            let reached = available.clone().unwrap_or_default();
            let out = self.statement(statement, &block[at + 1..], &reached);
            available = available.and(out);
        }
        available.map(|mut out| {
            out.retain(|search, _| entry.contains_key(search));
            out
        })
    }

    /// Plan `statement`, followed in its block by `rest` and reached with
    /// `before` available; what is available after it.
    fn statement(
        &mut self,
        statement: &AlgorithmStmt,
        rest: &[AlgorithmStmt],
        before: &Available,
    ) -> Flow {
        let own = searches_of(statement, self.keyed);
        // A loop reads its condition and its body on every turn: only what its
        // body does not kill is available there.
        let turn = |available: &Available| match statement {
            AlgorithmStmt::While { body, .. } | AlgorithmStmt::Foreach { body, .. } => {
                loop_entry(body, available, self.keyed)
            }
            _ => available.clone(),
        };
        let mut available = before.clone();
        let mut opened = Vec::new();
        for search in own.iter().collect::<BTreeSet<_>>() {
            if available.contains_key(search) {
                continue;
            }
            let mut with = available.clone();
            with.insert(search.clone(), String::new());
            if !turn(&with).contains_key(search) {
                continue;
            }
            if uses(search, statement, rest, self.keyed) < 2 {
                continue;
            }
            let local = format!("sce_found_{}", self.made);
            self.made += 1;
            available.insert(search.clone(), local.clone());
            opened.push(Opened {
                search: search.clone(),
                local,
                put_through: false,
            });
        }
        let reading = turn(&available);
        let through: BTreeMap<Search, String> = own
            .iter()
            .filter_map(|search| {
                reading
                    .get(search)
                    .map(|local| (search.clone(), local.clone()))
            })
            .collect();
        if let Some(search) = put_search(statement, self.keyed) {
            if let Some(local) = through.get(&search) {
                self.written.insert(local.clone());
            }
        }
        if !opened.is_empty() {
            self.plan.opened.insert(address(statement), opened);
        }
        if !through.is_empty() {
            self.plan.through.insert(address(statement), through);
        }
        match statement {
            AlgorithmStmt::If {
                then_body,
                else_body,
                ..
            } => {
                let then = self.block(then_body, &available);
                let otherwise = match else_body {
                    Some(body) => self.block(body, &available),
                    None => Some(available.clone()),
                };
                meet(then, otherwise)
            }
            AlgorithmStmt::While { body, .. } | AlgorithmStmt::Foreach { body, .. } => {
                self.block(body, &reading);
                Some(reading)
            }
            _ => after(statement, &available, self.keyed),
        }
    }
}

/// What is available on both of two paths; a path that does not go on
/// constrains nothing.
fn meet(one: Flow, other: Flow) -> Flow {
    match (one, other) {
        (None, flow) | (flow, None) => flow,
        (Some(one), Some(other)) => Some(
            one.into_iter()
                .filter(|(search, local)| other.get(search) == Some(local))
                .collect(),
        ),
    }
}

/// What of `available` a loop whose body is `body` is entered with on every
/// turn: the fixed point of its body's kills, which only remove.
fn loop_entry(
    body: &[AlgorithmStmt],
    available: &Available,
    keyed: &dyn Fn(&str) -> bool,
) -> Available {
    let mut turn = available.clone();
    loop {
        // A body that returns on every path turns no more: what it kills on the
        // way does not reach the next turn.
        let next = match transfer_block(body, &turn, keyed) {
            Some(out) => meet(Some(turn.clone()), Some(out)).unwrap_or_default(),
            None => turn.clone(),
        };
        if next == turn {
            return turn;
        }
        turn = next;
    }
}

/// What of `available` is available after `block`, no search made once.
fn transfer_block(
    block: &[AlgorithmStmt],
    available: &Available,
    keyed: &dyn Fn(&str) -> bool,
) -> Flow {
    let mut out = Some(available.clone());
    for statement in block {
        let available = out?;
        out = transfer(statement, &available, keyed);
    }
    out
}

/// What of `available` is available after `statement`, no search made once.
fn transfer(
    statement: &AlgorithmStmt,
    available: &Available,
    keyed: &dyn Fn(&str) -> bool,
) -> Flow {
    match statement {
        AlgorithmStmt::If {
            then_body,
            else_body,
            ..
        } => meet(
            transfer_block(then_body, available, keyed),
            match else_body {
                Some(body) => transfer_block(body, available, keyed),
                None => Some(available.clone()),
            },
        ),
        AlgorithmStmt::While { body, .. } | AlgorithmStmt::Foreach { body, .. } => {
            Some(loop_entry(body, available, keyed))
        }
        _ => after(statement, available, keyed),
    }
}

/// What of `available` a statement that holds no block leaves available: all
/// of it but what the statement kills (docs/adr/0161, D1); `None` after a
/// return, which no path goes on past.
fn after(statement: &AlgorithmStmt, available: &Available, keyed: &dyn Fn(&str) -> bool) -> Flow {
    if matches!(statement, AlgorithmStmt::Return { .. }) {
        return None;
    }
    let writes = Writes::of(statement);
    let put = put_search(statement, keyed);
    Some(
        available
            .iter()
            .filter(|(search, _)| {
                let map_moved = writes.reaches(&search.map) && put.as_ref() != Some(*search);
                !map_moved && !writes.touches(&search.key)
            })
            .map(|(search, local)| (search.clone(), local.clone()))
            .collect(),
    )
}

/// How many times `search` is made by `statement` and, while it stays
/// available, by what follows it in its block, `rest`.
fn uses(
    search: &Search,
    statement: &AlgorithmStmt,
    rest: &[AlgorithmStmt],
    keyed: &dyn Fn(&str) -> bool,
) -> usize {
    let mut available = Available::new();
    available.insert(search.clone(), String::new());
    let (made, out) = made_in(search, statement, &available, keyed);
    match out {
        Some(out) if out.contains_key(search) => made + made_in_block(search, rest, &out, keyed).0,
        _ => made,
    }
}

/// How many times `search`, available in `available`, is made in `block`
/// before it is killed, and what is available after it.
fn made_in_block(
    search: &Search,
    block: &[AlgorithmStmt],
    available: &Available,
    keyed: &dyn Fn(&str) -> bool,
) -> (usize, Flow) {
    let mut made = 0;
    let mut flow = Some(available.clone());
    for statement in block {
        let Some(available) = flow
            .as_ref()
            .filter(|available| available.contains_key(search))
        else {
            break;
        };
        let (here, out) = made_in(search, statement, available, keyed);
        made += here;
        flow = out;
    }
    (made, flow)
}

/// How many times `search`, available in `available`, is made by
/// `statement` and the blocks it holds, and what is available after it.
fn made_in(
    search: &Search,
    statement: &AlgorithmStmt,
    available: &Available,
    keyed: &dyn Fn(&str) -> bool,
) -> (usize, Flow) {
    let own = searches_of(statement, keyed)
        .iter()
        .filter(|made| *made == search)
        .count();
    match statement {
        AlgorithmStmt::If {
            then_body,
            else_body,
            ..
        } => {
            let (then, then_out) = made_in_block(search, then_body, available, keyed);
            let (otherwise, otherwise_out) = match else_body {
                Some(body) => made_in_block(search, body, available, keyed),
                None => (0, Some(available.clone())),
            };
            (own + then + otherwise, meet(then_out, otherwise_out))
        }
        AlgorithmStmt::While { body, .. } | AlgorithmStmt::Foreach { body, .. } => {
            let turn = loop_entry(body, available, keyed);
            if !turn.contains_key(search) {
                return (0, Some(turn));
            }
            (
                own + made_in_block(search, body, &turn, keyed).0,
                Some(turn),
            )
        }
        _ => (own, after(statement, available, keyed)),
    }
}

/// The search a `<sce:put>` makes of the map it writes, at its key.
pub(crate) fn put_search(
    statement: &AlgorithmStmt,
    keyed: &dyn Fn(&str) -> bool,
) -> Option<Search> {
    let AlgorithmStmt::Put { target, key, .. } = statement else {
        return None;
    };
    let map = target.trim();
    if !keyed(map) {
        return None;
    }
    let key = parse_to_ast(key.trim()).ok()?;
    Some(Search {
        map: map.to_string(),
        key: subject(&key)?,
    })
}

/// Every search `statement` makes itself, in its own expressions and its put,
/// once for each time it makes it; not what a block it holds makes. An
/// expression that does not parse makes none here: it is refused, with its
/// place, where it is lowered.
pub(crate) fn searches_of(statement: &AlgorithmStmt, keyed: &dyn Fn(&str) -> bool) -> Vec<Search> {
    let mut found = Vec::new();
    for text in own_expressions(statement) {
        if let Ok(tree) = parse_to_ast(text.trim()) {
            searches_in(&tree, keyed, &mut found);
        }
    }
    found.extend(put_search(statement, keyed));
    found
}

/// The expressions a statement reads itself, not those of a block it holds.
fn own_expressions(statement: &AlgorithmStmt) -> Vec<&str> {
    match statement {
        AlgorithmStmt::Var { init, .. } => init.iter().map(String::as_str).collect(),
        AlgorithmStmt::RecordVar { fields, .. } | AlgorithmStmt::UnionVar { fields, .. } => {
            fields.iter().map(|field| field.expr.as_str()).collect()
        }
        AlgorithmStmt::RecordFromCall { init, .. } | AlgorithmStmt::UnionCopied { init, .. } => {
            vec![init.as_str()]
        }
        AlgorithmStmt::Assign { expr, .. } | AlgorithmStmt::Append { expr, .. } => {
            vec![expr.as_str()]
        }
        AlgorithmStmt::Put { expr, .. } => expr.iter().map(String::as_str).collect(),
        AlgorithmStmt::If { cond, .. }
        | AlgorithmStmt::While { cond, .. }
        | AlgorithmStmt::Require { cond, .. } => vec![cond.as_str()],
        AlgorithmStmt::Return { expr, .. } => expr.iter().map(String::as_str).collect(),
        AlgorithmStmt::Call { args, .. } => args.iter().map(|arg| arg.expr.as_str()).collect(),
        AlgorithmStmt::Foreach { .. }
        | AlgorithmStmt::Remove { .. }
        | AlgorithmStmt::Clear { .. } => Vec::new(),
    }
}

/// The searches `node` makes: `has(m, k)` and `m[k]` of a map or set `m` the
/// body holds by name, at a key that is a name or a field of one.
fn searches_in(node: &TypedExpr, keyed: &dyn Fn(&str) -> bool, found: &mut Vec<Search>) {
    match &node.kind {
        ExprKind::Call { callee, args, .. }
            if matches!(&callee.kind, ExprKind::Ident(name) if name == "has")
                && args.len() == 2 =>
        {
            found.extend(search_of(&args[0], &args[1], keyed));
        }
        ExprKind::Index { object, index } => found.extend(search_of(object, index, keyed)),
        _ => {}
    }
    for child in node.children() {
        searches_in(child, keyed, found);
    }
}

/// The search of `map` at `key`, where `map` names a map or a set the body
/// holds and `key` is a name or a field of one.
pub(crate) fn search_of(
    map: &TypedExpr,
    key: &TypedExpr,
    keyed: &dyn Fn(&str) -> bool,
) -> Option<Search> {
    let ExprKind::Ident(map) = &map.kind else {
        return None;
    };
    if !keyed(map) {
        return None;
    }
    Some(Search {
        map: map.clone(),
        key: subject(key)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The body of an algorithm that declares `m`, a map of `uint32` to
    /// `uint32`, its first statement, and reads the `uint32`s `k` and `j`.
    fn body_of(body: &str) -> Vec<AlgorithmStmt> {
        let source = format!(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
  sce:kind="algorithm" name="probe" version="1.0">
  <sce:signature>
    <sce:param name="k" type="uint32"/>
    <sce:param name="j" type="uint32"/>
    <sce:return type="uint32" may-fail="true"/>
  </sce:signature>
  <sce:body><sce:var name="m" type="map&lt;uint32, uint32&gt;[4]"/>{body}</sce:body>
</scxml>"#
        );
        let parsed = crate::forge::parser::parse_forge_with_imports(
            &source,
            crate::DocumentLabel::symmetric("probe.scxml"),
        )
        .expect("the probe parses")
        .expect("the probe is a forge document");
        match parsed.document {
            crate::forge::model::ForgeDocument::Algorithm(m) => m.body,
            _ => panic!("the probe is an algorithm"),
        }
    }

    fn keyed(name: &str) -> bool {
        name == "m"
    }

    fn search(key: &str) -> Search {
        Search {
            map: "m".into(),
            key: key.into(),
        }
    }

    /// Every search the plan makes once, in the body's order, and the searches
    /// each statement reads through one, at any depth.
    fn planned(body: &[AlgorithmStmt]) -> (Vec<Opened>, Vec<Vec<Search>>) {
        let plan = SearchPlan::of(body, &keyed);
        let mut opened = Vec::new();
        let mut through = Vec::new();
        fn walk(
            block: &[AlgorithmStmt],
            plan: &SearchPlan,
            opened: &mut Vec<Opened>,
            through: &mut Vec<Vec<Search>>,
        ) {
            for statement in block {
                opened.extend(plan.opened_before(statement).iter().cloned());
                through.push(
                    plan.through(statement)
                        .map(|t| t.keys().cloned().collect())
                        .unwrap_or_default(),
                );
                for nested in crate::forge::narrowing::Statement::blocks(statement) {
                    walk(nested, plan, opened, through);
                }
            }
        }
        walk(body, &plan, &mut opened, &mut through);
        (opened, through)
    }

    /// The consumer's rule (docs/adr/0161, Context): a test, a read and a put
    /// of one key on every path search once, before the test, and the put goes
    /// through it.
    #[test]
    fn a_test_a_read_and_a_put_of_one_key_search_once() {
        let body = body_of(
            r#"<sce:if cond="has(m, k)">
  <sce:var name="last" type="uint32" init="m[k]"/>
  <sce:if cond="j > last"><sce:put target="m" key="k" expr="j"/></sce:if>
<sce:else><sce:put target="m" key="k" expr="j"/></sce:else>
</sce:if>
<sce:return expr="0"/>"#,
        );
        let (opened, through) = planned(&body);
        assert_eq!(
            opened,
            [Opened {
                search: search("k"),
                local: "sce_found_0".into(),
                put_through: true,
            }]
        );
        assert_eq!(
            through,
            [
                vec![],
                vec![search("k")],
                vec![search("k")],
                vec![],
                vec![search("k")],
                vec![search("k")],
                vec![],
            ]
        );
    }

    /// A branch that returns constrains nothing after it: a test and a read in
    /// a condition whose branch returns, and a put after it, search once
    /// (`algorithm_take_generation`).
    #[test]
    fn a_branch_that_returns_constrains_nothing_after_it() {
        let body = body_of(
            r#"<sce:if cond="has(m, k) &amp;&amp; m[k] &gt;= j"><sce:return expr="m[k]"/></sce:if>
<sce:put target="m" key="k" expr="j"/>
<sce:return expr="0"/>"#,
        );
        let (opened, through) = planned(&body);
        assert_eq!(opened.len(), 1, "{opened:?}");
        assert!(opened[0].put_through);
        assert_eq!(
            through,
            [
                vec![],
                vec![search("k")],
                vec![search("k")],
                vec![search("k")],
                vec![]
            ]
        );
    }

    /// A search made once is not made again where something between can move
    /// the key's entry: a remove of the map, a put of another key, a clear, a
    /// write of the key.
    #[test]
    fn a_remove_a_put_of_another_key_and_a_write_of_the_key_end_a_search() {
        let around = |between: &str| {
            body_of(&format!(
                r#"<sce:var name="x" type="uint32" init="k"/>
<sce:var name="y" type="uint32" init="j"/>
<sce:var name="a" type="uint32" init="m[x]"/>
{between}
<sce:var name="b" type="uint32" init="m[x]"/>
<sce:return expr="a + b + y"/>"#
            ))
        };
        for between in [
            r#"<sce:remove target="m" key="j"/>"#,
            r#"<sce:put target="m" key="j" expr="1"/>"#,
            r#"<sce:clear target="m"/>"#,
            r#"<sce:assign target="x" expr="j"/>"#,
        ] {
            let (opened, _) = planned(&around(between));
            assert!(opened.is_empty(), "{between}: {opened:?}");
        }
        // A put of the same key leaves it, and a write of another name does.
        for between in [
            r#"<sce:put target="m" key="x" expr="1"/>"#,
            r#"<sce:assign target="y" expr="k"/>"#,
        ] {
            let (opened, _) = planned(&around(between));
            assert_eq!(opened.len(), 1, "{between}: {opened:?}");
        }
    }

    /// One search of a key, or two of two keys, is made as it was: a search is
    /// made once only where another reads what it found.
    #[test]
    fn a_search_made_once_is_one_another_reads() {
        let body = body_of(
            r#"<sce:var name="a" type="uint32" init="m[k]"/>
<sce:var name="b" type="uint32" init="m[j]"/>
<sce:return expr="a + b"/>"#,
        );
        assert!(planned(&body).0.is_empty());
        let twice = body_of(r#"<sce:return expr="m[k] + m[k]"/>"#);
        assert_eq!(planned(&twice).0.len(), 1);
    }

    /// A loop is entered with what its body does not kill: a search made
    /// before it is read in its condition and its body where the body leaves
    /// the key's entry where it is, and not where the body moves it; a search
    /// a branch makes is not available after the branch.
    #[test]
    fn a_loop_and_a_branch_keep_what_their_blocks_do_not_kill() {
        let keeps = body_of(
            r#"<sce:var name="n" type="uint32" init="0"/>
<sce:if cond="has(m, k)"><sce:assign target="n" expr="1"/></sce:if>
<sce:while cond="n &lt; m[k]" max-iter="4"><sce:assign target="n" expr="n + 1"/></sce:while>
<sce:return expr="n"/>"#,
        );
        assert_eq!(planned(&keeps).0.len(), 1);
        let moves = body_of(
            r#"<sce:var name="n" type="uint32" init="0"/>
<sce:if cond="has(m, k)"><sce:assign target="n" expr="1"/></sce:if>
<sce:while cond="n &lt; m[k]" max-iter="4"><sce:remove target="m" key="j"/><sce:assign target="n" expr="n + 1"/></sce:while>
<sce:return expr="n"/>"#,
        );
        assert!(planned(&moves).0.is_empty());
        let branch = body_of(
            r#"<sce:var name="n" type="uint32" init="0"/>
<sce:if cond="j > 0"><sce:assign target="n" expr="m[k]"/></sce:if>
<sce:return expr="n + m[k]"/>"#,
        );
        assert!(planned(&branch).0.is_empty());
    }
}
