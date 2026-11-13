#!/usr/bin/env bash
# Run inside the L2 client container, on the client network, from the
# repository root. The discovery-l2 workflow injects the DISCOVERY_* and
# TOXIPROXY_* variables through the topology's env file. L2_PROFILE picks the
# rows: every row outside its profile skips itself, so the default profile
# runs the whole directory and the others run one file.
#
# L2_SITE names a directory holding the installed aerospike_native wheel,
# pytest and pytest-asyncio; the workflow fills it on the runner, for the
# container's Python version. Without it the container's own site-packages
# must already carry them. L2_FILTER, a pytest -k expression, replaces the
# profile's selection, to rerun single rows.
set -euo pipefail

host="${DISCOVERY_ENDPOINT_HOSTNAME:?DISCOVERY_ENDPOINT_HOSTNAME is required}"
ports="${DISCOVERY_ENDPOINT_PORTS:?DISCOVERY_ENDPOINT_PORTS is required}"
subset="${DISCOVERY_SUBSET:?DISCOVERY_SUBSET is required}"
profile="${L2_PROFILE:-default}"
suite=python/tests/discovery_l2

case "$subset" in
  ce|ee) ;;
  *)
    echo "DISCOVERY_SUBSET must be ce or ee (got: ${subset})" >&2
    exit 1
    ;;
esac

[[ "$host" =~ ^[A-Za-z0-9.-]+$ ]] || {
  echo "Refusing unexpected endpoint hostname: ${host}" >&2
  exit 1
}

case "$profile" in
  default) target="$suite" ;;
  pass-through-ports) target="$suite/pass_through_test.py" ;;
  cn-only) target="$suite/cn_only_test.py" ;;
  *)
    echo "Unknown profile ${profile}" >&2
    exit 1
    ;;
esac

if [[ "$subset" == "ee" ]]; then
  check_ports="${DISCOVERY_TLS_ENDPOINT_PORTS:?DISCOVERY_TLS_ENDPOINT_PORTS is required}"
else
  check_ports="$ports"
fi

# A cluster that never came up must fail here, not look like a skipped suite.
IFS=',' read -r -a port_list <<< "$check_ports"
if ((${#port_list[@]} == 0)); then
  echo "No proxy ports to check" >&2
  exit 1
fi
for port in "${port_list[@]}"; do
  [[ "$port" =~ ^[0-9]+$ ]] || {
    echo "Refusing unexpected endpoint port: ${port}" >&2
    exit 1
  }
  echo "Checking TCP ${host}:${port}"
  timeout 10 bash -c "echo >/dev/tcp/${host}/${port}"
done

if [[ -n "${AEROSPIKE_CA_CERT:-}" && ! -r "${AEROSPIKE_CA_CERT}" ]]; then
  echo "AEROSPIKE_CA_CERT ${AEROSPIKE_CA_CERT} is not readable" >&2
  exit 1
fi

if [[ -n "${L2_SITE:-}" ]]; then
  [[ -d "${L2_SITE}" ]] || {
    echo "L2_SITE ${L2_SITE} is not a directory" >&2
    exit 1
  }
  export PYTHONPATH="${L2_SITE}${PYTHONPATH:+:${PYTHONPATH}}"
fi
python3 -c "import aerospike_native, pytest, pytest_asyncio" || {
  echo "aerospike_native, pytest and pytest-asyncio must be importable (set L2_SITE)" >&2
  exit 1
}

# Append mode keeps the checkout's python/ source tree behind the installed
# wheel on sys.path, so the import finds the compiled extension. No cache: the
# workspace is bind-mounted and the container runs as root. The rows print
# client warnings themselves as CLIENT-LOG lines, so live logging would print
# each one twice.
args=(-v -rfEsxX --import-mode=append -p no:cacheprovider -o log_cli=false)
if [[ -n "${L2_FILTER:-}" ]]; then
  args+=(-k "${L2_FILTER}")
  target="$suite"
fi

echo "Running python3 -m pytest ${target} ${args[*]}"
exec python3 -m pytest "$target" "${args[@]}"
