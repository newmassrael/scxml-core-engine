# Deploying the relay of a shared list

The relay (`docs/SCE_SHARED_LIST_RELAY.md`, ADR 0034) runs as one program, `sce-relay`
(`backends/rust/relay-server`), on a small Linux server: one static binary, one configuration file, a
key, a certificate when it terminates TLS itself, and a store directory. This document is the runbook
for a server of 1 GB of memory and a fraction of a core, managed by systemd, with firewalld and SELinux
enforcing. Nothing in it names a particular host; replace `relay.example` with the relay's own name.

| What | Where on the server | Made by |
|---|---|---|
| The program | `/usr/local/bin/sce-relay` | a build elsewhere, copied (section 1) |
| The configuration | `/etc/sce-relay/relay.toml` | `backends/rust/relay-server/deploy/relay.toml`, edited |
| The authority's key | `/etc/sce-relay/authority.key`, mode 0600, root's | `sce-relay keygen` (section 3) |
| The certificate chain and its key | `/etc/sce-relay/tls/chain.pem`, `/etc/sce-relay/tls/key.pem` (0600) | an authority (section 4) |
| The store | `/var/lib/sce-relay` (systemd's `StateDirectory=`) | the relay |
| The service | `/etc/systemd/system/sce-relay.service` | `deploy/sce-relay.service` |
| The firewall's service | `/etc/firewalld/services/sce-relay.xml` | `deploy/sce-relay.xml` |

## 1. Build a static binary and copy it

The binary is built on a development machine for `x86_64-unknown-linux-musl`, which links everything
it needs into one file that runs on any x86_64 Linux whatever its C library. `ring` (the TLS
cryptography) compiles C and assembly, so the build needs a C compiler for the musl target; `musl-gcc`
(the `musl-tools` package of Debian and Ubuntu) is one, and the host's own `gcc` also serves, since
`ring`'s C includes no header of the C library (`CC_x86_64_unknown_linux_musl=gcc`, built so
2026-11-02):

```sh
rustup target add x86_64-unknown-linux-musl
CC_x86_64_unknown_linux_musl=musl-gcc \
  cargo build --release --locked --target x86_64-unknown-linux-musl \
  -p sce-rust-relay-server --bin sce-relay
file target/x86_64-unknown-linux-musl/release/sce-relay   # "static-pie linked" or "statically linked"
```

Copy it to the server and install it with `install`, which makes a new file with the label SELinux
gives that directory; a file moved with `mv` from a home directory keeps the home directory's label,
and systemd is then refused when it runs it:

```sh
scp target/x86_64-unknown-linux-musl/release/sce-relay admin@relay.example:/tmp/
ssh admin@relay.example
sudo install -o root -g root -m 0755 /tmp/sce-relay /usr/local/bin/sce-relay
sudo restorecon -v /usr/local/bin/sce-relay      # SELinux: the label of /usr/local/bin
sce-relay --version
```

Under SELinux the relay runs in the domain systemd gives a service with no policy of its own
(`unconfined_service_t`), which may bind port 443 and write `/var/lib`; no policy module is needed.
Files put in `/etc/sce-relay` by `mv` need `restorecon -Rv /etc/sce-relay` for the same reason as the
binary.

## 2. What the program does

| Command | What it does | Exit status |
|---|---|---|
| `sce-relay serve --config FILE [flags]` | runs the relay until SIGTERM or SIGINT | 0 when stopped |
| `sce-relay check --config FILE [flags]` | reads the configuration, the keys, the certificate and the whole store as `serve` does, and exits | 0 when all of it holds |
| `sce-relay keygen --out FILE --audience NAME` | makes the authority's key, mode 0600, never over a file that is there | 0 |
| `sce-relay token issue --key FILE --list NAME --node N [--valid-for SECONDS]` | prints a token for one device of one list (section 6) | 0 |
| `sce-relay probe URL (--key FILE \| --token-file FILE) [--ca FILE] [--timeout SECONDS]` | makes one real exchange with a running relay and prints one line: what it came to and how long each stage took (section 11) | 0, or the status of the stage that failed (section 11) |

`serve` and `check` take the configuration file and, over it, `--listen`, `--listen-tls`,
`--tls-chain`, `--tls-key`, `--store`, `--authority-key`, `--max-connections` and `--limit NAME=VALUE`
(`sce-relay serve --help`). No secret is ever given on the command line: only the paths of the files
that hold them. A key file, and the certificate's private key, are refused when anyone but their
owner may read or change them, except in the directory of credentials systemd hands a service
(`$CREDENTIALS_DIRECTORY`), which only that service enters and where systemd may grant the service's
user its read by an access-control entry that shows as a group bit; the file the credential is copied
from is then the operator's to keep at 0600.

What a status says, so that the service manager can tell a relay that will not start whatever it does
from one that may start later:

| Status | Meaning | Started again by systemd |
|---|---|---|
| 0 | stopped as it was told | no |
| 2 | a command line the program does not take | no |
| 65 | the store holds what does not verify, or a file a store does not hold | no (`RestartPreventExitStatus`) |
| 69 | a listening address could not be taken (in use) | yes |
| 70 | the program could not start its runtime | yes |
| 73 | a key file could not be made (one is there already) | n/a |
| 74 | a token could not be written out | n/a |
| 78 | the configuration, a key, the certificate or the store's directory is wrong or unreadable | no (`RestartPreventExitStatus`) |

Every line the relay writes is one line on stderr, which the journal keeps (`journalctl -u
sce-relay`): `level=info event=listening transport=wss address=[::]:443`, `event=ready`,
`event=stopping signal=SIGTERM`, `event=stopped accepted=... refused=... unopened=... requests=...`,
and `level=error event=refused status=78 reason="..."` naming the file or setting that is wrong. A line
names counts, addresses, paths and reasons; never a key, a token or anything a list holds.

Each decision of the relay itself is a line of the same form too, with the module that made it as
`target`: every refusal, with its code and reason (`event=refused code=forbidden reason="..."`); every
refusal for a limit, naming the limit, how long until the device may ask again and the counts behind it
(`event=rate_limited limit=per_device_allowance op=submit cost=2 burst=120 refill_per_second=20
retry_after_ms=50 list=1a2b3c4d node=2`, and `connections_per_minute`, `challenges_per_minute`,
`logins_per_minute`, `logins_per_node_per_hour`, `joins_per_minute`, `creations_per_minute`,
`creations_per_hour`, `used_nonces`, `sources` for a table that holds as many sources as it may, and a
connection with no token closed at `anonymous_requests` or `anonymous_idle`); every login admitted,
token issued, list created, join kept, entry of a log appended and sweep of lapsed lists; and a store
that did not take a record, at `error`. A list is named by a digest of its name (`list=1a2b3c4d`), never
by its name. How much is written is `SCE_RELAY_LOG` in the unit's environment
(`Environment=SCE_RELAY_LOG=debug`): `info` unless it says otherwise; `debug` adds a line for every
request answered as asked and every connection closed; `warning`, `error` and `off` write less. A word it
does not read is said once (`event=log_level_unknown`) and `info` is written. The libraries the relay
links (TLS, WebSocket, the runtime) are written at `warning` and `error` only, at every level: their
`debug` and `trace` hold what the relay's own lines never write, a frame's payload among it.

## 3. The authority's key

The authority's key signs every token the relay takes (`docs/SCE_SHARED_LIST_RELAY.md` section 2). Make
it on the server, as root:

```sh
sudo mkdir -p /etc/sce-relay/tls
sudo sce-relay keygen --out /etc/sce-relay/authority.key --audience relay.example
```

The audience is the relay's name, written into every token, so that a token for one relay is no use at
another. The file is three lines (`sce-relay-authority 1`, `audience <name>`, `secret <32 bytes>`) and
is only ever copied whole, to the service that issues tokens (section 6), never printed or pasted. The
unit hands it to the relay as a credential (`LoadCredential=`), so the relay's own user never reads
`/etc`.

**Changing the key** (it leaked, or on a schedule): make a new one with `keygen` to a new file, move it
over the old, give it to the issuer, and restart the relay. Every token of the old key is refused from
then on, and each device logs in again (section 6) or asks the issuer for a new one on the refusal, as
it does when one expires.

## 4. TLS

A device dials `wss://relay.example/sce-relay/1`. Three ways to give the relay a certificate:

1. **A public certificate, the relay terminating TLS** (what the files in `deploy/` do). An authority
   the devices already believe issues it, for instance by ACME (`certbot certonly --standalone -d
   relay.example`, which needs port 80 open while it runs, or a DNS challenge). The chain
   (`fullchain.pem`: the server's certificate, then the intermediates) goes to
   `/etc/sce-relay/tls/chain.pem` and the key (`privkey.pem`) to `/etc/sce-relay/tls/key.pem`, mode
   0600. The certificate is read when the relay starts: a renewal is followed by `systemctl restart
   sce-relay` (a deploy hook of the ACME client). The devices' library believes Mozilla's public
   authorities, compiled in (`WssLink::new`, and the peer core's `sce_peer_link_wss`), so nothing
   changes on a phone.
2. **A private authority of the deployment's own.** The deployment makes an authority and issues the
   relay's certificate from it:

   ```sh
   openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:P-256 -nodes -days 3650 \
     -subj "/CN=relay deployment authority" -keyout ca.key -out ca.pem
   openssl req -newkey ec -pkeyopt ec_paramgen_curve:P-256 -nodes \
     -subj "/CN=relay.example" -keyout key.pem -out relay.csr
   openssl x509 -req -in relay.csr -CA ca.pem -CAkey ca.key -CAcreateserial -days 825 \
     -extfile <(printf "subjectAltName=DNS:relay.example\nextendedKeyUsage=serverAuth") \
     -out chain.pem
   ```

   A device must then believe that authority: in Rust, `WssLink::believing` with
   `tls::Roots::Only(vec![ca_der])`; through the peer core's C ABI, `sce_peer_link_wss_believing` with
   the authority's certificate as PEM or DER (`ca.pem`, shipped in the application) and
   `SCE_PEER_ROOTS_ONLY`, or `SCE_PEER_ROOTS_AND_PUBLIC` for a deployment that will move its relays to
   a public authority without stranding the applications it has shipped (`PeerLink.wssBelieving` in
   the Kotlin binding).
3. **A proxy that terminates TLS** (nginx, Caddy, a load balancer) in front of a relay that listens for
   `ws://` on `127.0.0.1:8780` (`[listen] plain`, no `[tls]` section; drop `--tls-key`, the
   `tls.key` credential and the two capability lines from the unit). The proxy must pass the
   WebSocket upgrade and the `Authorization` header through, and must not buffer. On a server of 1 GB,
   a second process costs memory the relay could use; prefer option 1 there.

## 5. Configure, check, start

```sh
sudo install -m 0644 relay.toml /etc/sce-relay/relay.toml          # from deploy/, edited
sudo install -m 0644 sce-relay.service /etc/systemd/system/
sudo sce-relay check --config /etc/sce-relay/relay.toml \
  --authority-key /etc/sce-relay/authority.key --tls-key /etc/sce-relay/tls/key.pem \
  --store /tmp/sce-relay-check
sudo systemctl daemon-reload
sudo systemctl enable --now sce-relay
systemctl status sce-relay
journalctl -u sce-relay -n 20
```

The `check` before the first start is given a store of its own (`--store` in a scratch directory) so that
the state directory is made by systemd, for the service's user, and not by root. The relay is
`Type=notify`: systemd counts it as started only once it has read and verified its store
and taken its port, and it says `STOPPING=1` when it is told to stop, so `systemctl start` returns when
the relay answers and an order that waits on it waits for a relay that does. Starting reads every record
of the store again; `TimeoutStartSec=300` is the time a large store is given.

On SIGTERM (`systemctl stop`, `restart`, a shutdown) the relay stops accepting, closes each connection
that waits for a request, lets each one that is answering finish its answer, waits up to
`shutdown_grace_seconds` for all of them, and exits 0. Nothing needs flushing: the relay acknowledges a
record only after it is on the disk.

The unit is hardened: a user made for the service (`DynamicUser=`), the one directory it writes
(`StateDirectory=`), its keys as credentials, no privilege but binding port 443, a read-only system, no
other user's files, a system-call filter, and `MemoryMax=640M` for a server of 1 GB.

**The configuration's keys.** Every key is optional but where the table says otherwise; a section or
a key the relay does not read is refused by name, with its line.

| Section | Keys (default) |
|---|---|
| `[listen]` | `plain`, `tls` (addresses with ports; one of them at least), `trusted_proxies` (none) |
| `[tls]` | `certificate_chain`, `private_key` (both, or neither; with `[listen] tls`) |
| `[store]` | `directory` (required), `slots` (1) |
| `[authority]` | `key_file` (required) |
| `[connections]` | `max` (512), `handshake_seconds` (10), `idle_seconds` (300), `shutdown_grace_seconds` (10), `anonymous_requests` (8), `anonymous_idle_seconds` (30) |
| `[limits]` | `burst`, `refill_per_second`, `write_cost`, `max_request_bytes`, `max_feed`, `max_range`, `max_records_per_list`, `max_lists`, `max_group_message_bytes`, `max_key_packages_per_node` (section 9) |
| `[login]` | `enabled` (true), `token_seconds` (3600), `connections_per_minute`, `challenges_per_minute`, `logins_per_minute` (30 each), `logins_per_node_per_hour` (60), `joins_per_minute` (30), `joins_outstanding_per_list` (32) (section 6) |
| `[creation]` | `policy` (`"closed"`), `lapse_days` (90), `work_bits` (20), `creations_per_minute` (4), `creations_per_hour` (60) (section 6) |

## 6. Tokens: who issues a device's token

A device presents a token naming one list and its node in that list (`docs/SCE_SHARED_LIST_RELAY.md`
section 2). Two ways issue one, and a deployment may use both.

**The relay issues a member's token itself (ADR 0080), by default.** A device that is a member of a
list logs in on a connection with no token: it signs the relay's challenge with the key the list's
membership log gave its node, and the relay, which holds the log, checks the signature and issues a
token of its authority for `[login] token_seconds` (an hour). No service and no operator is in the
loop, and the relay learns no account. A device whose token expired logs in again the same way. The
relay draws the key of its challenges at start, so a restart voids every challenge it gave, and
none of it is kept on disk.

```toml
[listen]
# Behind a proxy that terminates TLS: the proxy's address, whose X-Forwarded-For names a connection's
# source. A header from any other address is not believed.
trusted_proxies = ["127.0.0.1"]

[connections]
anonymous_requests = 8          # what a connection with no token may ask before it is closed
anonymous_idle_seconds = 30

[login]
enabled = true                  # false: every device presents a token of the authority
token_seconds = 3600
connections_per_minute = 30     # per source: an IPv4 address, or an IPv6 /64
challenges_per_minute = 30
logins_per_minute = 30
logins_per_node_per_hour = 60
joins_per_minute = 30           # per source: a device joins under an administrator's invitation
joins_outstanding_per_list = 32 # joins of one list waiting for an administrator to admit them
```

A source that opens more connections with no token than it may is refused at the upgrade (HTTP 429,
with `Retry-After`), and one that asks more challenges, logins or joins is answered `rate_limited`.
A device an administrator invited joins on the same kind of connection (ADR 0080 section 4): it
leaves its key package under the administrator's signed invitation, which the relay checks against
the log it holds, and is issued no token until it is a member; a list that holds
`joins_outstanding_per_list` joins whose invitation has not expired and whose node is not granted
refuses the next (`too_large`). Nothing is configured for it beyond these bounds. Behind a
proxy that terminates TLS, name the proxy in `trusted_proxies`, or every device is one source, the
proxy's. What logs in, from where and when, is learned by the relay as it learns it of every
connection; it logs counts, never a key, a token or a nonce. A member taken off is issued no new
token; one it holds still connects until it expires, and every request is judged by the log.

**Who creates a list: `[creation] policy`.** The first entry of a list the relay does not hold is
taken under the deployment's creation policy (ADR 0080 section 3):

```toml
[creation]
policy = "closed"               # "closed" (the default) or "open"
lapse_days = 90                 # a list created openly that no login touched this long is removed
work_bits = 20                  # "open": the proof of work a creation takes, 1 to 24
creations_per_minute = 4        # "open": per source
creations_per_hour = 60         # "open": of the whole relay
```

- **`closed`** (the default, and the recommendation for any deployment with a service of any kind): a
  list is created by a device that presents a token of the authority for its name, which the
  deployment's service or an operator issues (below). A token for a name the relay does not hold is
  the creation ticket; nothing else is configured.
- **`open`**, for an application with no service at all: any device may create a list on a connection
  with no token. The relay's challenge for a list it does not hold names the work (`work_bits`), and
  the device's login sends the list's first entries with a counter that meets it, about a million
  hashes at 20 bits, a second or less on a phone. The list's name must be `n1.<salt>.<digest>`, the
  digest of the key the list's genesis names first (`sce_peer_certified_name`), so that no one takes a
  name another device made. A source makes `creations_per_minute`, the relay takes
  `creations_per_hour`, and `[limits] max_lists` bounds the lists as for any list. A list created so
  **lapses**: one no login touched for `lapse_days` (and an hour, the step a login's touch is written
  in) is removed from the store, whole, and a device that still holds it creates it again on its next
  login with the same name, and gives it back what it holds. `open` is refused at start without
  `[login] enabled = true` and with `lapse_days = 0`: a list created openly that never lapsed would be
  one a stranger keeps for good.

The relay removes the lists that lapsed as it starts (`event=lapsed removed=N kept=M`) and every hour
after (a line only when it removed one). A list created with a token never lapses, whatever the
policy; `lapse_days = 0` turns lapse off where nothing is created openly. The residual of `open` is a
stranger who spends the work to make lists up to `max_lists`, each held until it lapses: keep
`max_lists` at what the server holds (section 9), and `creations_per_hour` low. What the relay learns
of a creation is the list's name, its node and the source's prefix, as of any login; it logs counts.

A device's side needs nothing of the operator: the host names the list (`sce_peer_certified_name`),
opens it (`sce_peer_open_list`) and logs in (`sce_peer_login`), which creates it at a relay whose
policy is `open` (`docs/SCE_SHARED_LIST_PEER_ABI.md` section 13).

**Whoever holds the authority's key issues tokens the relay takes.** In a deployment with a sign-in
service of its own, that is the service: it authenticates the person by its own means (an identity
provider, a password, a passkey), decides which device of which list the request is for (the node the
list's membership log gave that device), and issues a token good for a short time (an hour). The device
presents it; when the relay answers `unauthorized` (the token expired, or the key changed) the device
asks the service again. No operator is in the loop for any device, and the relay learns no account: it
knows a device by its node in one list.

- The service holds a copy of the key file. A Rust service reads it with
  `Authority::from_key_text` and issues with `Authority::issue(list, node, expires_at_ms)`; a service
  in another language writes the same bytes (the format and a known-answer vector are in
  `docs/SCE_SHARED_LIST_RELAY.md` section 2), or runs `sce-relay token issue`.
- Who may **create** a list on the relay is who the service issues a token for a list name the relay
  holds none of; `[limits] max_lists` bounds how many there can be whatever the service does.
- `sce-relay token issue` is the same issuance from the command line: the token of the first
  administrator's device before a service exists, a probe that checks the relay answers, a service
  written as a script. It prints the token on standard output and logs only the list, the node and the
  expiry.
- The key is symmetric (HMAC-SHA256): the service can issue and the relay can check, and so can the
  relay issue. That gives a relay nothing it lacks, since a token is good only at the relay of its
  audience. What a stolen key gives is bounded by the rest of the design: a token lets a device read
  what the log lets its node read and spend that node's allowance; what is kept is decided by the
  members' signatures and the log, which no token changes.

An application with no service of its own has its members' devices log in and its administrators
invite (above), and either gives each list's first entries with a token of the authority (`closed`,
an operator's act once a list) or runs the relay `open`, where no token of the authority is needed
at all. A deployment whose service issues every token may turn login off (`[login] enabled = false`),
and keeps creation `closed`.

## 7. Firewall

```sh
sudo install -m 0644 sce-relay.xml /etc/firewalld/services/
sudo firewall-cmd --reload
sudo firewall-cmd --permanent --zone=public --add-service=sce-relay
sudo firewall-cmd --reload
sudo firewall-cmd --zone=public --list-services
```

The service opens the port the relay listens on with TLS (443). A relay behind a proxy opens nothing of
its own; the proxy's port is opened. Port 80 is opened only while an ACME client needs it.

## 8. Upgrade

```sh
sudo install -m 0755 /tmp/sce-relay /usr/local/bin/sce-relay.new
sudo restorecon -v /usr/local/bin/sce-relay.new
sudo /usr/local/bin/sce-relay.new check --config /etc/sce-relay/relay.toml \
  --authority-key /etc/sce-relay/authority.key --tls-key /etc/sce-relay/tls/key.pem \
  --store /var/lib/sce-relay
sudo mv /usr/local/bin/sce-relay.new /usr/local/bin/sce-relay
sudo systemctl restart sce-relay
```

`check` on the live store reads it and writes nothing (it writes only the directories of a store that
was never started). Devices see the restart as a connection lost and dial again on their next round;
nothing a device sent is lost, since a device keeps what it signed and gives the relay what its heads
show it lacks. Keep the old binary until the new one has run.

**A store from before lapse** (a relay built before 2026-11-03) holds no list's mark: every list in
it was created with a token of the authority, and none of them ever lapses. Nothing is converted, and
the new relay reads it as it is; `check` names the `creation` policy and `lapse_days` it would run
with. Turning `open` on later changes nothing for those lists. A relay started with `open` and then
stopped and downgraded to a build from before lapse would read each list created openly as one
created with a token (it does not read the marks) and keep it for good: do not downgrade past it, or
remove the lists marked `.open` first. **The configuration** gains `[creation]` (section 6), every key
of it optional; a file of before reads as `closed` with lapse of 90 days, which lapses nothing.

## 9. Memory on a server of 1 GB

The relay keeps every record of every list it holds in memory and verifies them all again when it
starts; it never forgets one (compaction is not built). What it can come to is about
`max_lists x max_records_per_list x` the size of a record (a write or a note travels as some hundreds
of bytes sealed, a membership entry or a checkpoint of the same order, an MLS message up to
`max_group_message_bytes`), plus the buffers of each open connection. The defaults of the library
suit one household and are far beyond a server of 1 GB if every bound were reached; `deploy/relay.toml`
sets `max_lists = 200` and `max_records_per_list = 50000`. Measure the relay under the load it will see
(`cargo run --release -p sce-rust-relay --features serve --example load`, and `systemctl status
sce-relay` for the memory it holds) and set the bounds so that their product fits under `MemoryHigh`.
A list that reaches its bound refuses writes (`too_large`); a relay that reaches `max_lists` refuses new
lists and keeps the ones it holds.

## 10. Backup and restore

The store is a directory of append-only files: `slot-<n>/` for each slot, and in it one file for each
list, named by the hexadecimal of the list's name, a line for each record. Every line is a record the
relay verified, and the relay verifies each again when it starts. A list created with no token has a
second file of the same name with the extension `.open`, one line that says when the list was created
and when a login last touched it; it is replaced whole, never appended to. A list that lapses loses
both files, the only thing the relay ever deletes.

- **Backup.** A copy taken while the relay runs is a store: each file is a prefix of the list's
  records, and a last line cut off by the copy is dropped when it is read. No list's records depend on
  another's. As root:

  ```sh
  sudo tar -C /var/lib/private -czf /root/sce-relay-$(date +%F).tar.gz sce-relay
  ```

  (`/var/lib/sce-relay` is a link to `/var/lib/private/sce-relay` under `DynamicUser=`.) The copy holds
  no key and nothing a list's members sealed is readable in it; it does hold the lists' names, the
  nodes and the stamps of writes, so it is kept like the server's own data.
- **Check a copy** without touching the relay: unpack it elsewhere and run `sce-relay check --config
  /etc/sce-relay/relay.toml --authority-key /etc/sce-relay/authority.key --tls-key
  /etc/sce-relay/tls/key.pem --store /path/to/the/copy`.
- **Restore.** `systemctl stop sce-relay`, replace the directory's contents with the copy, `sce-relay
  check` as above, `systemctl start sce-relay`. A relay restored from an older copy holds less than
  its devices do; each device's next round compares heads and gives the relay what it lacks, so what
  any device still holds comes back. What only the relay held after the copy (a member who wrote and
  has since lost the device) does not.
- **One relay per store.** Two relays must never run on one store directory at once; under systemd one
  unit is one process.
- **Slots.** The number of slots is fixed when a store begins. To change it, stop the relay, move each
  list's files (its `.jsonl`, and its `.open` when it has one) into the directory of the slot its name
  belongs to for the new number (`sce_rust_relay::slot_of`), set `[store] slots`, and `check`: a relay
  whose store is laid out for another number refuses to start and names the file, a mark included.
- **A restored copy and lapse.** A copy restored holds the marks as they were when it was taken: a
  list that lapsed after the copy comes back and lapses again at the next sweep, and one whose marks
  say an older login lapses sooner than it would have. Its devices create it again on their next
  login in either case.

## 11. Health

The relay answers on its port once systemd shows it `active (running)`. Whether it answers a device is
another question, which `sce-relay probe` asks from anywhere a device could be: it makes the exchange a
device makes, stage by stage, and prints one line.

```sh
# On the server, from the authority's key: the probe issues itself a token for five minutes.
sudo sce-relay probe wss://relay.example --key /etc/sce-relay/authority.key
# From a monitoring host, which holds a token and not the key: the token is made on the server into
# a file only its owner reads (the probe refuses one others may read, as it refuses such a key), and
# the file is copied to the monitor with its mode kept.
(umask 077; sudo sce-relay token issue --key /etc/sce-relay/authority.key \
  --list sce-relay-probe --node 1 --valid-for 31536000 > probe.token)
sce-relay probe wss://relay.example --token-file probe.token [--ca ca.pem] [--timeout 10]
```

```
outcome=ok relay=wss://relay.example/sce-relay/1 asked=write_read list=held resolve_ms=1.2 connect_ms=21.4 tls_ms=44.0 upgrade_ms=22.1 exchange_ms=23.0 total_ms=112.0
outcome=tls_refused relay=wss://relay.example/sce-relay/1 asked=write_read stage=tls resolve_ms=1.1 connect_ms=20.9 total_ms=40.3 reason="the TLS handshake failed: invalid peer certificate: Expired"
```

The stages are `resolve` (the name), `connect`, `tls` (a `wss://` relay, believing the public
authorities, or with `--ca` the one in that file alone), `upgrade` (the WebSocket upgrade with the token,
which the relay judges) and `exchange`. What it asks depends on the token. A token for the probe list,
`sce-relay-probe` (the one `--key` issues, or one issued for that list as above), lets it write: it gives
the relay that list's first entry, the same entry every time, which the first probe makes the relay keep
(`list=created`) and every later one finds held (`list=held`), and reads the log back to check it. The
store is written once and the list never grows. A relay that says `list=created` again after the first
probe has lost its store. A token for any other list is used only to read that list's log
(`asked=read`): a probe never writes to an application's list. The line never holds the token.

| Status | `outcome=` | What failed |
|---|---|---|
| 0 | `ok` | nothing: every request was answered as the protocol says, and what was written read back |
| 69 | `unreachable` | the name did not resolve, or nothing took the connection in time |
| 79 | `tls_refused` | the TLS handshake: an authority the probe does not believe, another name, an expired certificate, or no TLS at the address (`sysexits.h` has no number for it: 79 is the first past its last) |
| 77 | `token_refused` | the relay refused the token, at the upgrade or as `unauthorized`: another relay's, expired, or the key changed |
| 75 | `busy` | the relay took the connection and did not finish a stage within `--timeout`, closed it before it answered (it holds as many connections as it takes, or it is stopping), or answered `rate_limited` |
| 74 | `store_failed` | the relay answered `unavailable`: its store did not take a record, and it takes nothing until its operator has looked (`journalctl -u sce-relay`) |
| 76 | `protocol_error` | the relay answered, but not as the protocol says: the upgrade refused for another reason (a proxy that does not pass it), a text that is not a response, or a read back that is not what was written |
| 78 | none | the probe's own inputs: the key, the token file or the `--ca` file cannot be read, or another user may read the key or the token |
| 2 | none | the command line: no token and no key, both, no time, or an address that is not `ws://` or `wss://` |

A probe's write spends its node's allowance like a device's (a write and a read each run); a probe every
minute is far inside the defaults. The probe list is one of the relay's lists (`max_lists`); it is
created with a token, and never lapses. A probe presents a token and does not log in. A probe that
logged in was considered with ADR 0080's step 5 and not built: the probe list's administrator key is
public by design, so a login to it is one anyone could take over by bringing a grant of its own, and
a sound one needs a list of its own under keys derived from the authority's, given to the relay with
a token first (ADR 0080, "Status of the plan"). To watch that login works, a monitor logs a member's
device in (`docs/SCE_SHARED_LIST_PEER_ABI.md` section 5).

The relay does not use systemd's watchdog: the answer of a request is the liveness worth checking, and a
ping from a thread of the process says nothing of it.

## 12. Open decisions for the owner

- **Tokens for an application with no service of its own** (decided: ADR 0080, accepted
  2026-11-02, built 2026-11-02 and 2026-11-03). A member's device logs in and the relay issues its
  token, an administrator invites a device by a signed invitation under which it leaves its key
  package, and a list is created under `[creation] policy` (`closed` by default, `open` with proof of
  work and lapse) (section 6). A probe that logs in is not built (section 11).
- **A private authority on a phone** waits on the ABI call that names one (section 4, option 2).
- **Reloading a renewed certificate** without a restart is not built: a restart takes seconds and the
  devices dial again.
