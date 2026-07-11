#!/usr/bin/env bash
set -euo pipefail

PORTS=("$@")
if [ "${#PORTS[@]}" -eq 0 ]; then
    PORTS=(5432 6379)
fi

kill_by_port() {
    local port="$1"
    echo "Looking for listener on port ${port}..."
    local line
    line=$(ss -ltnp "sport = :${port}" | tail -n +2 | head -n 1 || true)
    if [ -z "${line}" ]; then
        echo "No listener on port ${port}."
        return 0
    fi

    local pid
    pid=$(echo "${line}" | sed -n 's/.*pid=\([0-9]\+\).*/\1/p')
    if [ -z "${pid}" ]; then
        echo "Could not extract pid from ss output:"
        echo "${line}"
        echo "Try running this script with sudo to see process details."
        return 1
    fi

    echo "Listener PID: ${pid}"
    local parent_pid
    parent_pid=$(ps -o ppid= -p "${pid}" | tr -d ' ')
    echo "Parent PID: ${parent_pid}"
    ps -o pid,ppid,user,cmd -p "${pid}" "${parent_pid}" || true

    local ppcmd
    local pidcmd
    ppcmd=$(ps -o cmd= -p "${parent_pid}" | tr -d '\n')
    pidcmd=$(ps -o cmd= -p "${pid}" | tr -d '\n')

    if echo "${ppcmd}" | grep -q "containerd-shim"; then
        local cid
        cid=$(echo "${ppcmd}" | sed -n 's/.*-id \([a-f0-9]\+\).*/\1/p')
        if [ -n "${cid}" ]; then
            echo "containerd id: ${cid}"
            for ns in moby default; do
                echo "Trying namespace: ${ns}"
                sudo ctr -n "${ns}" tasks kill -s SIGKILL "${cid}" || true
                sudo ctr -n "${ns}" tasks rm "${cid}" || true
                sudo ctr -n "${ns}" containers rm "${cid}" || true
            done
        else
            echo "containerd-shim found but no -id. Aborting."
            return 1
        fi
    elif echo "${pidcmd}" | grep -q "docker-proxy"; then
        local container_ip
        container_ip=$(echo "${pidcmd}" | sed -n 's/.*-container-ip \([0-9\.]*\).*/\1/p')
        if [ -z "${container_ip}" ]; then
            echo "docker-proxy found but no container IP. Aborting."
            return 1
        fi
        echo "docker-proxy container IP: ${container_ip}"
        local cid
        cid=$(docker network ls -q | xargs docker network inspect | jq -r --arg ip "${container_ip}" '..|.Containers? // empty | to_entries[]? | select(.value.IPv4Address|startswith($ip + "/")) | .key' | head -n 1)
        if [ -n "${cid}" ]; then
            echo "Stopping container: ${cid}"
            docker stop "${cid}" || true
            docker rm "${cid}" || true
        else
            echo "No container found for IP ${container_ip}. Killing docker-proxy PID ${pid}."
            kill "${pid}" || true
        fi
    elif echo "${pidcmd}" | grep -q "com.docker.backend"; then
        echo "Detected Docker Desktop backend listener."
        local docker_config="${DOCKER_CONFIG:-}"
        if [ -z "${docker_config}" ] && [ -n "${SUDO_USER:-}" ] && [ -d "/home/${SUDO_USER}/.docker" ]; then
            docker_config="/home/${SUDO_USER}/.docker"
        fi
        local docker_cmd=(docker)
        if [ -n "${docker_config}" ]; then
            docker_cmd=(env DOCKER_CONFIG="${docker_config}" docker)
        fi

        local contexts
        contexts=$("${docker_cmd[@]}" context ls --format '{{.Name}}' 2>/dev/null || true)
        if [ -z "${contexts}" ]; then
            contexts="default"
        fi

        for ctx in ${contexts}; do
            echo "Searching context: ${ctx}"
            local match
            match=$("${docker_cmd[@]}" --context "${ctx}" ps --format "{{.ID}}\t{{.Names}}\t{{.Ports}}" | grep "${port}->" || true)
            if [ -n "${match}" ]; then
                local cid
                cid=$(echo "${match}" | head -n 1 | awk '{print $1}')
                echo "Stopping container in ${ctx}: ${cid}"
                "${docker_cmd[@]}" --context "${ctx}" stop "${cid}" || true
                "${docker_cmd[@]}" --context "${ctx}" rm "${cid}" || true
            fi
        done
    else
        echo "Process is not containerd-shim, docker-proxy, or Docker Desktop backend. Aborting."
        return 1
    fi

    echo "Done. Port status:"
    ss -ltnp "sport = :${port}" || true
}

for port in "${PORTS[@]}"; do
    kill_by_port "${port}"
done
