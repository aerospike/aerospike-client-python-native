# Copyright 2023-2026 Aerospike, Inc.
#
# Portions may be licensed to Aerospike, Inc. under one or more contributor
# license agreements WHICH ARE COMPATIBLE WITH THE APACHE LICENSE, VERSION 2.0.
#
# Licensed under the Apache License, Version 2.0 (the "License"); you may not
# use this file except in compliance with the License. You may obtain a copy of
# the License at http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
# WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
# License for the specific language governing permissions and limitations under
# the License.

"""The L2 topology as the environment describes it.

A three node cluster on a private network with no published ports,
reachable from the client only through one Toxiproxy listener per node
behind one endpoint hostname. The client has no route to the node addresses.

- ``DISCOVERY_ENDPOINT_HOSTNAME``: hostname every listener is behind
- ``DISCOVERY_ENDPOINT_PORTS``: clear listener ports, one per node
- ``DISCOVERY_TLS_ENDPOINT_PORTS``: TLS listener ports; empty on ``ce``
- ``DISCOVERY_PROXY_MAP``: Toxiproxy proxy name to listener port
- ``DISCOVERY_SUBSET``: ``ce`` or ``ee``
- ``TOXIPROXY_URL``: Toxiproxy control API
- ``AEROSPIKE_CONTAINER_NAMES``: server containers, one per node
- ``DISCOVERY_CONTROL_DIR``: directory shared with the host, which starts and
  removes nodes on request; empty when the host offers no node control
- ``DISCOVERY_REACHABLE_NODE``: ``reachable`` profile, the node advertising an
  address the client can reach
- ``DISCOVERY_PASS_THROUGH_BRANCH``: ``A`` when each node advertises its own
  alternate-access-address hostname, ``B`` when every node advertises one
  hostname with its own listener port
- ``DISCOVERY_PORT_MAP_RANGE``, ``DISCOVERY_PORT_MAP_PORT_BASE``:
  ``port-map-range`` profile, a ``first-last`` range of node addresses with a
  listener each
- ``DISCOVERY_TLS_CERT_SHAPE``: ``san-only`` or ``cn-only`` on ``ee``
- ``DISCOVERY_CLIENT_NETWORK_MTU``: ``low-mtu`` profile
- ``DISCOVERY_PROXY_PORT_RANGE``: ``port-range`` profile, ``first last``
- ``DISCOVERY_IDLE_RESET``: ``idle-reset`` profile
- ``AEROSPIKE_CA_CERT``: the server CA on ``ee``

Each node's name and advertised addresses are read through its listener at
runtime.
"""

import asyncio
import functools
import http.client
import inspect
import json
import os
import socket
import time
import urllib.parse
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Callable, Optional

from aerospike_native import Client, ClientPolicy, Host, MetricsPolicy, Node

TEND_INTERVAL = 250
CONNECT_TIMEOUT = 1000
WAIT_MS = 30_000
TLS_PROXY_SUFFIX = "-tls"
PARTITIONS = 4096
INFO_TIMEOUT = 5.0
HTTP_TIMEOUT = 10.0
RUN_HINT = "Run the discovery-l2 workflow, which builds the topology and sets it."


@dataclass(frozen=True)
class Topology:
    """The L2 topology the environment names."""

    host: str
    ports: list[int]
    tls_ports: list[int]
    proxy_ports: list[tuple[str, int]]
    subset: str
    toxiproxy_url: str
    containers: list[str]
    control_dir: str
    reachable_node: str
    pass_through_branch: str
    port_map_range: str
    port_map_port_base: str
    tls_cert_shape: str
    client_network_mtu: str
    proxy_port_range: str
    idle_reset: str
    ca_cert: str
    namespace: str


def _ports(csv: str) -> list[int]:
    return [int(p) for p in csv.split(",") if p]


@functools.cache
def topology() -> Optional[Topology]:
    """The topology, ``None`` when ``DISCOVERY_ENDPOINT_HOSTNAME`` is not set."""
    host = os.environ.get("DISCOVERY_ENDPOINT_HOSTNAME", "")
    if not host:
        return None

    def required(name: str) -> str:
        value = os.environ.get(name, "")
        assert value, f"Environment variable {name} is not set. {RUN_HINT}"
        return value

    def optional(name: str) -> str:
        return os.environ.get(name, "")

    proxy_ports = []
    for entry in required("DISCOVERY_PROXY_MAP").split(","):
        name, _, port = entry.rpartition(":")
        proxy_ports.append((name, int(port)))
    return Topology(
        host=host,
        ports=_ports(required("DISCOVERY_ENDPOINT_PORTS")),
        tls_ports=_ports(optional("DISCOVERY_TLS_ENDPOINT_PORTS")),
        proxy_ports=proxy_ports,
        subset=required("DISCOVERY_SUBSET"),
        toxiproxy_url=required("TOXIPROXY_URL"),
        containers=required("AEROSPIKE_CONTAINER_NAMES").split(","),
        control_dir=optional("DISCOVERY_CONTROL_DIR"),
        reachable_node=optional("DISCOVERY_REACHABLE_NODE"),
        pass_through_branch=optional("DISCOVERY_PASS_THROUGH_BRANCH"),
        port_map_range=optional("DISCOVERY_PORT_MAP_RANGE"),
        port_map_port_base=optional("DISCOVERY_PORT_MAP_PORT_BASE"),
        tls_cert_shape=optional("DISCOVERY_TLS_CERT_SHAPE"),
        client_network_mtu=optional("DISCOVERY_CLIENT_NETWORK_MTU"),
        proxy_port_range=optional("DISCOVERY_PROXY_PORT_RANGE"),
        idle_reset=optional("DISCOVERY_IDLE_RESET"),
        ca_cert=optional("AEROSPIKE_CA_CERT"),
        namespace=os.environ.get("AEROSPIKE_NAMESPACE", "test"),
    )


def t() -> Topology:
    """The topology; the row has checked it exists."""
    found = topology()
    assert found is not None, f"DISCOVERY_ENDPOINT_HOSTNAME is not set. {RUN_HINT}"
    return found


def namespace() -> str:
    return t().namespace


@dataclass(frozen=True)
class L2Node:
    """Node behind one listener, as it advertises itself."""

    proxy: str
    port: int
    name: str
    advertised: list[str]
    tls_port: int = 0
    tls_advertised: list[str] = field(default_factory=list)
    tls_name: Optional[str] = None

    def __str__(self) -> str:
        text = f"{self.name} proxy={self.proxy} listener={self.port} advertised={self.advertised}"
        if self.tls_port:
            text += (
                f" tlsListener={self.tls_port} tlsAdvertised={self.tls_advertised}"
                f" tlsName={self.tls_name}"
            )
        return text


@dataclass(frozen=True)
class Peer:
    """Peer as listed by a node's peers reply."""

    name: str
    tls_name: str
    endpoints: list[str]


_nodes: Optional[list[L2Node]] = None


async def nodes() -> list[L2Node]:
    """Queries every listener for its node's name and advertised addresses.

    Read once per run.
    """
    global _nodes
    if _nodes is not None:
        return _nodes
    topo = t()
    assert len(topo.containers) == len(topo.ports), (
        f"DISCOVERY_ENDPOINT_PORTS {topo.ports} vs AEROSPIKE_CONTAINER_NAMES {topo.containers}"
    )
    commands = ["node", "service-clear-std"]
    if topo.tls_ports:
        commands += ["service-tls-std", "peers-tls-std"]

    found = []
    tls_names: dict[str, str] = {}
    for port in topo.ports:
        proxy = _proxy_name(port)
        reply = await info(topo.host, port, commands)
        name = reply.get("node")
        assert name, f"No node name from listener {port}"
        tls_port = next(
            (p for n, p in topo.proxy_ports if n == f"{proxy}{TLS_PROXY_SUFFIX}"), 0
        )
        node = L2Node(
            proxy=proxy,
            port=port,
            name=name,
            advertised=endpoints(reply.get("service-clear-std")),
            tls_port=tls_port,
            tls_advertised=endpoints(reply.get("service-tls-std")),
        )
        assert node.advertised, f"Node {name} advertises no clear address"
        found.append(node)
        for peer in parse_peers(reply.get("peers-tls-std")):
            if peer.tls_name:
                tls_names[peer.name] = peer.tls_name
    _nodes = []
    for node in found:
        node = L2Node(**{**node.__dict__, "tls_name": tls_names.get(node.name)})
        print(f"L2-NODE {node}")
        _nodes.append(node)
    return _nodes


async def node_names() -> list[str]:
    """The cluster's node names, sorted."""
    return sorted(n.name for n in await nodes())


async def seed() -> Host:
    """The first listener as a seed."""
    return Host(t().host, (await nodes())[0].port)


def listener(port: int) -> str:
    return f"{t().host}:{port}"


def policy() -> ClientPolicy:
    """The policy every row starts from: a fast tend and a short connect timeout."""
    p = ClientPolicy()
    p.tend_interval = TEND_INTERVAL
    p.timeout = CONNECT_TIMEOUT
    p.fail_if_not_connected = True
    return p


def port_map_by_endpoint(node_list: list[L2Node]) -> dict[str, str]:
    """port-map from each node's advertised clear ``host:port`` to its listener."""
    return {a: listener(n.port) for n in node_list for a in n.advertised}


def port_map_by_node_name(node_list: list[L2Node]) -> dict[str, str]:
    """port-map from each node's name to its listener."""
    return {n.name: listener(n.port) for n in node_list}


def tls_port_map(node_list: list[L2Node]) -> dict[str, str]:
    """port-map from each node's advertised TLS ``host:port`` to its TLS listener."""
    return {a: listener(n.tls_port) for n in node_list for a in n.tls_advertised}


def client_node_names(client: Client) -> list[str]:
    """The client's node names, sorted."""
    return sorted(n.name for n in client.nodes())


async def wait_for(condition: Callable[[], Any], poll_ms: int = 100) -> bool:
    """Polls ``condition`` every ``poll_ms`` for up to ``WAIT_MS``.

    ``condition`` may be a coroutine function. An assertion it raises fails
    the row at once.
    """
    deadline = time.monotonic() + WAIT_MS / 1000
    while True:
        result = condition()
        if inspect.isawaitable(result):
            result = await result
        if result:
            return True
        if time.monotonic() >= deadline:
            return False
        await asyncio.sleep(poll_ms / 1000)


async def wait_nodes(client: Client, names: list[str]) -> None:
    """Waits until the client's node names are exactly ``names``."""
    await wait_for(lambda: client_node_names(client) == names)
    assert client_node_names(client) == names


def masters(client: Client) -> list[Optional[Node]]:
    """The master of every partition of the namespace in the client's map,
    an inactive node counting as none."""
    found = client._partition_masters(namespace())
    if found is None:
        return [None] * PARTITIONS
    result = [n if n is not None and n.is_active else None for n in found]
    return result + [None] * (PARTITIONS - len(result))


def unowned(master_list: list[Optional[Node]]) -> int:
    return sum(1 for m in master_list if m is None)


async def assert_all_partitions_owned(row: str, client: Client) -> list[Optional[Node]]:
    """Waits until every partition of the namespace has an active master in
    the client's partition map and returns the master of each partition."""
    await wait_for(lambda: unowned(masters(client)) == 0)
    master_list = masters(client)
    missing = unowned(master_list)
    print(f"L2-PARTITIONS {row} partitions={PARTITIONS} unowned={missing} owners={owners(master_list)}")
    assert missing == 0
    return master_list


def owners(master_list: list[Optional[Node]]) -> dict[str, int]:
    """The number of partitions each node is master of, by node name."""
    counts: dict[str, int] = {}
    for node in master_list:
        if node is not None:
            counts[node.name] = counts.get(node.name, 0) + 1
    return dict(sorted(counts.items()))


def owners_of(client: Client) -> dict[str, int]:
    return owners(masters(client))


async def assert_unreachable(endpoint: str) -> None:
    """Asserts the client cannot open a TCP connection to ``endpoint``."""
    try:
        _, writer = await asyncio.wait_for(
            asyncio.open_connection(host_of(endpoint), port_of(endpoint)), 2
        )
    except asyncio.TimeoutError:
        print(f"L2-UNREACHABLE {endpoint} connect timed out")
        return
    except OSError as e:
        print(f"L2-UNREACHABLE {endpoint} {e}")
        return
    writer.close()
    raise AssertionError(f"Advertised address {endpoint} is reachable from the client")


def messages(error: BaseException) -> str:
    """The error's message, which carries its cause chain."""
    return str(error)


# ---------------------------------------------------------------------------
# Info protocol
# ---------------------------------------------------------------------------


def info_request(commands: list[str]) -> bytes:
    """An info request for ``commands``, as a client sends it."""
    payload = "".join(f"{c}\n" for c in commands).encode()
    return bytes([2, 1]) + len(payload).to_bytes(6, "big") + payload


async def read_info_reply(reader: asyncio.StreamReader) -> dict[str, str]:
    header = await reader.readexactly(8)
    body = await reader.readexactly(int.from_bytes(header[2:], "big"))
    reply = {}
    for line in body.decode(errors="replace").splitlines():
        key, _, value = line.partition("\t")
        reply[key] = value
    return reply


async def info(host: str, port: int, commands: list[str]) -> dict[str, str]:
    """One info request to ``host:port``; raises when it does not complete."""

    async def request() -> dict[str, str]:
        reader, writer = await asyncio.open_connection(host, port)
        try:
            writer.write(info_request(commands))
            await writer.drain()
            return await read_info_reply(reader)
        finally:
            writer.close()

    try:
        return await asyncio.wait_for(request(), INFO_TIMEOUT)
    except asyncio.TimeoutError:
        raise AssertionError(f"info {host}:{port} {commands}: timed out") from None


async def info_value(node: L2Node, command: str) -> str:
    """One info value read through a node's listener."""
    return (await info(t().host, node.port, [command])).get(command, "")


async def namespace_stats(node: L2Node, names: list[str]) -> dict[str, int]:
    """The namespace statistics of a node, read through its listener."""
    reply = await info_value(node, f"namespace/{namespace()}")
    stats = {}
    for token in reply.split(";"):
        name, sep, value = token.partition("=")
        if sep and name in names:
            stats[name] = int(value) if value.lstrip("-").isdigit() else 0
    for wanted in names:
        assert wanted in stats, f"Node {node.name} has no namespace statistic {wanted}"
    return stats


async def peers_generation(node: L2Node) -> int:
    return int(await info_value(node, "peers-generation"))


async def alternates(node: L2Node) -> list[str]:
    """The alternate ``host:port`` list a node advertises in service-clear-alt."""
    return endpoints(await info_value(node, "service-clear-alt"))


async def peer_name(observer: L2Node, command: str, endpoint: str) -> Optional[str]:
    """The name of the peer ``observer`` lists with ``endpoint`` in a peers reply."""
    names = await peer_names(observer, command, endpoint)
    return names[0] if names else None


async def peer_names(observer: L2Node, command: str, endpoint: str) -> list[str]:
    """The names of the peers ``observer`` lists with ``endpoint`` in ``command``."""
    peers = parse_peers(await info_value(observer, command))
    return sorted(p.name for p in peers if endpoint in p.endpoints)


async def wait_cluster_stable(size: int) -> None:
    """Waits until every node reports the same cluster key for a cluster of
    ``size`` nodes with no migrations left."""
    command = f"cluster-stable:size={size};ignore-migrations=false"
    node_list = await nodes()
    deadline = time.monotonic() + WAIT_MS / 1000
    while True:
        keys = []
        for node in node_list:
            try:
                keys.append((await info(t().host, node.port, [command])).get(command, ""))
            except (OSError, AssertionError, asyncio.IncompleteReadError) as e:
                keys.append(str(e))
        stable = len(set(keys)) == 1 and keys[0] and not keys[0].startswith("ERROR")
        if stable or time.monotonic() >= deadline:
            print(f"L2-CLUSTER-STABLE size={size} {keys}")
            assert stable, f"Cluster not stable at size {size}: {keys}"
            return
        await asyncio.sleep(0.1)


def parse_peers(reply: Optional[str]) -> list[Peer]:
    """Parses a peers-clear-std or peers-tls-std reply:
    ``gen,port,[[node,tls-name,[address[:port],...]],...]``."""
    if not reply:
        return []
    parts = reply.split(",", 2)
    if len(parts) < 3:
        return []
    default_port = int(parts[1]) if parts[1].isdigit() else 0
    entries = parts[2].strip()
    if entries.startswith("[") and entries.endswith("]"):
        entries = entries[1:-1]
    peers = []
    for entry in _split_top_level(entries):
        entry = entry.strip()
        if not (entry.startswith("[") and entry.endswith("]")):
            continue
        fields = entry[1:-1].split(",", 2)
        if len(fields) < 3:
            continue
        name, tls_name, addresses = fields
        addresses = addresses.strip()
        if not (addresses.startswith("[") and addresses.endswith("]")):
            continue
        found = []
        for address in _split_top_level(addresses[1:-1]):
            address = address.strip()
            if not address:
                continue
            has_port = "]:" in address if address.startswith("[") else ":" in address
            found.append(address if has_port else f"{address}:{default_port}")
        peers.append(Peer(name=name, tls_name=tls_name, endpoints=found))
    return peers


def _split_top_level(text: str) -> list[str]:
    """Splits ``text`` at the commas outside brackets."""
    parts, depth, current = [], 0, ""
    for c in text:
        if c == "[":
            depth += 1
        elif c == "]":
            depth = max(0, depth - 1)
        elif c == "," and depth == 0:
            parts.append(current)
            current = ""
            continue
        current += c
    if current:
        parts.append(current)
    return parts


def endpoints(reply: Optional[str]) -> list[str]:
    """A ``service-*`` reply as ``host:port`` strings, an IPv6 host in brackets."""
    if not reply:
        return []
    found = []
    for entry in (e.strip() for e in reply.split(",")):
        if not entry:
            continue
        host, port = _split_host_port(entry)
        found.append(f"[{host}]:{port}" if ":" in host else f"{host}:{port}")
    return found


def _split_host_port(entry: str) -> tuple[str, int]:
    """``host:port``, ``[ipv6]:port``, a bare host (port 3000) or a bare IPv6 address."""
    if entry.startswith("["):
        host, sep, after = entry[1:].partition("]")
        if sep:
            port = after[1:] if after.startswith(":") else ""
            return host, int(port) if port.isdigit() else 3000
    if entry.count(":") > 1:
        return entry, 3000
    host, sep, port = entry.rpartition(":")
    if not sep:
        return entry, 3000
    return host, int(port) if port.isdigit() else 3000


def host_of(endpoint: str) -> str:
    """The host of a ``host:port`` endpoint, brackets removed."""
    return _split_host_port(endpoint)[0]


def port_of(endpoint: str) -> int:
    return _split_host_port(endpoint)[1]


def resolve(name: str) -> list[str]:
    """The addresses ``name`` resolves to."""
    try:
        return sorted({a[4][0] for a in socket.getaddrinfo(name, None)})
    except OSError:
        return []


def assert_address(advertised: str, host: tuple[str, int]) -> None:
    """Asserts a node's ``(name, port)`` host is the advertised ``host:port``,
    by name or by one of the addresses the name resolves to."""
    name = host_of(advertised)
    addresses = resolve(name)
    assert host[0] == name or host[0] in addresses, f"{host[0]} is not {name}{addresses}"
    assert host[1] == port_of(advertised)


def _proxy_name(port: int) -> str:
    for name, p in t().proxy_ports:
        if p == port:
            return name
    raise AssertionError(f"DISCOVERY_PROXY_MAP has no proxy for listener {port}: {t().proxy_ports}")


# ---------------------------------------------------------------------------
# Toxiproxy
# ---------------------------------------------------------------------------


def _http(method: str, path: str, body: Optional[dict]) -> tuple[int, str]:
    url = urllib.parse.urlsplit(t().toxiproxy_url)
    connection = http.client.HTTPConnection(url.hostname, url.port or 80, timeout=HTTP_TIMEOUT)
    try:
        payload = json.dumps(body) if body is not None else None
        connection.request(method, path, body=payload, headers={"Content-Type": "application/json"})
        response = connection.getresponse()
        return response.status, response.read().decode()
    finally:
        connection.close()


async def toxiproxy(method: str, path: str, body: Optional[dict] = None) -> tuple[int, str]:
    """One request to the Toxiproxy control API: status and body."""
    return await asyncio.to_thread(_http, method, path, body)


async def set_listener(proxy: str, enabled: bool) -> None:
    """Enables or disables a Toxiproxy listener."""
    status, body = await toxiproxy("POST", f"/proxies/{proxy}", {"enabled": enabled})
    print(f"L2-TOXIPROXY {proxy} enabled={enabled} status={status} {body}")
    assert status == 200, f"{proxy}: {body}"


async def add_listener(proxy: str, upstream: str) -> int:
    """Creates a Toxiproxy listener on the first free port above every
    listener in use and returns that port."""
    port = await next_listener_port()
    await add_listener_on(proxy, port, upstream)
    return port


async def next_listener_port() -> int:
    """The first free port above every Toxiproxy listener in use."""
    return max(await listener_ports(), default=0) + 1


async def add_listener_on(proxy: str, port: int, upstream: str) -> None:
    """Creates a Toxiproxy listener on ``port``."""
    status, body = await toxiproxy(
        "POST",
        "/proxies",
        {"name": proxy, "listen": f"0.0.0.0:{port}", "upstream": upstream, "enabled": True},
    )
    print(f"L2-TOXIPROXY add {proxy} status={status} {body}")
    assert status == 201, f"{proxy}: {body}"


async def remove_listener(proxy: str) -> None:
    status, body = await toxiproxy("DELETE", f"/proxies/{proxy}")
    print(f"L2-TOXIPROXY remove {proxy} status={status}")
    assert status == 204, f"{proxy}: {body}"


async def add_toxic(
    proxy: str, name: str, kind: str, stream: str, toxicity: float, attributes: dict
) -> None:
    """Adds a toxic to a Toxiproxy listener."""
    status, body = await toxiproxy(
        "POST",
        f"/proxies/{proxy}/toxics",
        {"name": name, "type": kind, "stream": stream, "toxicity": toxicity, "attributes": attributes},
    )
    print(f"L2-TOXIPROXY toxic add {proxy} {name} status={status} {body}")
    assert status == 200, f"{proxy}: {body}"


async def remove_toxic(proxy: str, name: str) -> None:
    status, body = await toxiproxy("DELETE", f"/proxies/{proxy}/toxics/{name}")
    print(f"L2-TOXIPROXY toxic remove {proxy} {name} status={status}")
    assert status == 204, f"{proxy}: {body}"


async def upstream_port(proxy: str) -> int:
    """The port a Toxiproxy listener forwards to."""
    status, body = await toxiproxy("GET", f"/proxies/{proxy}")
    assert status == 200, f"{proxy}: {body}"
    return port_of(json.loads(body)["upstream"])


async def listener_ports() -> set[int]:
    """Every Toxiproxy listener port in use."""
    status, body = await toxiproxy("GET", "/proxies")
    assert status == 200, body
    ports = {p for _, p in t().proxy_ports}
    ports.update(int(p["listen"].rpartition(":")[2]) for p in json.loads(body).values())
    return ports


# ---------------------------------------------------------------------------
# Node control
# ---------------------------------------------------------------------------


async def control(command: str) -> dict[str, str]:
    """Asks the host to act on the server cluster through
    ``DISCOVERY_CONTROL_DIR`` and returns its acknowledgement.

    The request is written as ``<id>.request`` and the host answers with
    ``<id>.ack``, one ``key=value`` per line.
    """
    directory = Path(t().control_dir)
    request_id = f"{os.getpid()}-{time.time_ns()}"
    tmp = directory / f"{request_id}.tmp"
    ack = directory / f"{request_id}.ack"
    tmp.write_text(f"{command}\n")
    tmp.rename(directory / f"{request_id}.request")
    assert await wait_for(ack.exists), f"No acknowledgement for '{command}' in {directory}"
    reply = {}
    for line in ack.read_text().splitlines():
        key, sep, value = line.partition("=")
        if sep:
            reply[key] = value
    ack.unlink(missing_ok=True)
    print(f"L2-CONTROL {command} {reply}")
    assert "error" not in reply, f"'{command}' failed: {reply['error']}"
    return reply


@dataclass(frozen=True)
class Joined:
    """Node that joined the client and the time from the peers-generation
    bump that announced it to the node being active."""

    node: Node
    bump_to_active_ms: int
    generation: int


async def wait_joined(client: Client, observer: L2Node, generation: int, names: list[str]) -> Joined:
    """Waits for the peers-generation of ``observer`` to move from
    ``generation`` and for an active node not in ``names`` to appear in the
    client."""
    deadline = time.monotonic() + WAIT_MS / 1000
    bump: Optional[float] = None
    active: Optional[tuple[float, Node]] = None
    while True:
        now = time.monotonic()
        if bump is None and await peers_generation(observer) != generation:
            bump = now
        if active is None:
            node = next((n for n in client.nodes() if n.is_active and n.name not in names), None)
            if node is not None:
                active = (now, node)
        if bump is not None and active is not None:
            return Joined(
                node=active[1],
                bump_to_active_ms=max(0, int((active[0] - bump) * 1000)),
                generation=await peers_generation(observer),
            )
        assert time.monotonic() < deadline, (
            f"No peers-generation bump or no active node after generation {generation}"
        )
        await asyncio.sleep(0.005)


# ---------------------------------------------------------------------------
# Connection statistics
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class ConnStats:
    """The connection and error counters of one node."""

    in_use: int = 0
    in_pool: int = 0
    opened: int = 0
    closed: int = 0
    errors: int = 0
    timeouts: int = 0


def metrics_policy() -> MetricsPolicy:
    """Operational metrics, so command errors are counted."""
    p = MetricsPolicy()
    p.operational = True
    return p


def node_snapshot(client: Client, node: Node):
    """The metrics snapshot of ``node``, ``None`` before its first one."""
    name, port = node.host
    return client.metrics().nodes.get(f"{name}:{port}")


def connection_stats(client: Client) -> dict[str, ConnStats]:
    """The connection and error counters of every node, by node name.

    Counters accumulate while metrics are enabled; the pool gauges are live.
    """
    stats = {}
    for node in client.nodes():
        s = node_snapshot(client, node)
        stats[node.name] = ConnStats() if s is None else ConnStats(
            in_use=s.connections_in_use,
            in_pool=s.connections_in_pool,
            opened=s.connections_successful,
            closed=s.connections_closed,
            errors=s.transaction_error_count,
            timeouts=s.connections_timeout_errors,
        )
    return dict(sorted(stats.items()))


def format_stats(stats: dict[str, ConnStats]) -> str:
    return " ".join(
        f"{name}{{inUse={s.in_use},inPool={s.in_pool},opened={s.opened},"
        f"closed={s.closed},errors={s.errors},timeouts={s.timeouts}}}"
        for name, s in stats.items()
    )


# ---------------------------------------------------------------------------
# Restoring the topology after a row
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class EnableListener:
    proxy: str


@dataclass(frozen=True)
class RemoveListener:
    proxy: str


@dataclass(frozen=True)
class StopNode:
    container: str


@dataclass(frozen=True)
class RemoveToxic:
    proxy: str
    name: str


_restore: list = []
_clients: list[Client] = []


def register_client(client: Client) -> None:
    """Remembers a client the row opened, so it is closed after the row even
    when the row fails before closing it; a leaked client keeps tending and
    logging into the next row's capture."""
    _clients.append(client)


def restore_later(restore) -> None:
    """Registers something to undo after the row, whether it passes or fails."""
    _restore.append(restore)


def restored(kind: type) -> None:
    """Drops the registered restorations of ``kind`` the row has undone itself."""
    _restore[:] = [r for r in _restore if not isinstance(r, kind)]


async def restore_all() -> None:
    """Closes the row's clients and undoes, in order, what the row changed."""
    clients, _clients[:] = list(_clients), []
    for client in clients:
        if client.is_connected():
            await client.close()
    pending, _restore[:] = list(_restore), []
    stopped = False
    for restore in pending:
        if isinstance(restore, EnableListener):
            await set_listener(restore.proxy, True)
        elif isinstance(restore, RemoveListener):
            await remove_listener(restore.proxy)
        elif isinstance(restore, StopNode):
            await control(f"stop {restore.container}")
            stopped = True
        elif isinstance(restore, RemoveToxic):
            await remove_toxic(restore.proxy, restore.name)
    if stopped:
        await wait_cluster_stable(len(await nodes()))
