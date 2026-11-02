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
then on, and each device asks the issuer for a new one on the refusal, as it does when one expires.

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

## 6. Tokens: who issues a device's token

A device presents a token naming one list and its node in that list (`docs/SCE_SHARED_LIST_RELAY.md`
section 2). **Whoever holds the authority's key issues tokens the relay takes.** In a deployment that
is the application's sign-in service: it authenticates the person by its own means (an identity
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

An application with no service of its own cannot issue tokens without an operator. Section 12 records
the open decision.

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
relay verified, and the relay verifies each again when it starts.

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
  list's file into the directory of the slot its name belongs to for the new number
  (`sce_rust_relay::slot_of`), set `[store] slots`, and `check`: a relay whose store is laid out for
  another number refuses to start and names the file.

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
minute is far inside the defaults. The probe list is one of the relay's lists (`max_lists`).

The relay does not use systemd's watchdog: the answer of a request is the liveness worth checking, and a
ping from a thread of the process says nothing of it.

## 12. Open decisions for the owner

- **Tokens for an application with no service of its own.** Section 6 needs a service that holds the
  key. Without one, the textbook way is that the relay issues a member's token itself against a proof
  of possession: the device signs a fresh challenge with the key the list's log gave its node, and the
  relay checks the signature against the log it holds. It needs no person and no account, and it
  covers only members. Creating a list and a joining device's key package (both before the device is
  in the log) would need an admission rule as well: an administrator's signed admission of a node the
  relay checks against the log, or creation open to anyone up to `max_lists`. That is an addition to
  the protocol, the peer core and its C ABI, and a policy, and is not built. ADR 0080 (Proposed)
  sets out the design for the owner to accept or amend.
- **A private authority on a phone** waits on the ABI call that names one (section 4, option 2).
- **Reloading a renewed certificate** without a restart is not built: a restart takes seconds and the
  devices dial again.
