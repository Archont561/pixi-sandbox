---
type: Playbook
title: "Reproducing These Measurements"
description: The exact probe commands, so every number in this bundle can be re-derived by a reader with access to the same box.
resource: https://github.com/Archont561/pixi-sandbox
tags: [environment, reproduction]
status: stable
confidence: verified
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
legacy: { files: [`SANDBOX_CONSTRAINTS.md`], sections: ["12"] }
sources:
  - { id: pypiorg-simple-, resource: http://pypi.org/simple/, title: pypi.org/simple/ }
  - { id: debdebianorg--inrelease, resource: http://deb.debian.org/.../InRelease, title: deb.debian.org/.../InRelease }
  - { id: debdebianorg--inrelease, resource: http://deb.debian.org/.../InRelease, title: deb.debian.org/.../InRelease }
---

# Reproducing These Measurements

## 12. Reproducing these results

```bash
# identity + machine
uname -a; cat /etc/os-release; lscpu | head -20; nproc
free -h; df -h /; df -i /; ulimit -a; cat /proc/self/cgroup; cat /sys/fs/cgroup/user/cpu.max

# network model
ip -brief addr; ip route; cat /etc/resolv.conf; cat /etc/hosts; ss -tulnp
echo | openssl s_client -connect github.com:443 -servername github.com 2>/dev/null | grep -E 'issuer|subject'

# egress probe (000 == blocked)
for h in github.com api.github.com codeload.github.com registry.npmjs.org pypi.org \
         index.crates.io static.crates.io prefix.dev conda.anaconda.org pixi.sh \
         sh.rustup.rs bun.sh raw.githubusercontent.com release-assets.githubusercontent.com ghcr.io; do
  printf "%-40s %s\n" "$h" "$(curl -s -o /dev/null -m 8 -w '%{http_code}' https://$h/)"
done

# toolchain
for t in git gh curl python3 node npm pixi cargo rustc rustup bun go docker; do
  printf "%-8s " "$t"; command -v $t || echo ABSENT; done
```

<details>
<summary>Appendix: verbatim probe transcript (selected)</summary>

```
=== UNAME ===   Linux e2b.local 6.1.158+ #1 SMP PREEMPT_DYNAMIC Mon May 11 18:48:24 UTC 2026 x86_64 GNU/Linux
Mem: 3.8Gi total, 225Mi used, 3.7Gi free, Swap: 0B
/dev/root 21G 813M 20G 4% /
nameserver 8.8.8.8
uid=1001(user) gid=1001(user) groups=1001(user),27(sudo),100(users)
sudo -n id  ->  uid=0(root) gid=0(root) groups=0(root)
gh auth status -> Logged in to github.com as arena-ai-coding-agent[bot] (GH_TOKEN)

github.com                       https=200
codeload.github.com              https=301   (tarball: 200, 19939716 B)
api.github.com                   https=200   (core limit 5000, used 0)
registry.npmjs.org               https=200
pypi.org                         https=200
files.pythonhosted.org           https=404   (reachable)
index.crates.io / static.crates.io  https=000   curl: (35) SSL_ERROR_SYSCALL
prefix.dev / conda.anaconda.org     https=000
raw.githubusercontent.com             https=000
release-assets.githubusercontent.com  https=000  (blocks pixi binary download)
ghcr.io                               https=000
pixi.sh / sh.rustup.rs / bun.sh / gitlab.com  https=000
docs.astro.build / starlight.astro.build      https=000  (re-measured 2026-09-19)
quantco.github.io / *.github.io               https=000  (GitHub Pages unreadable from an airlock)
nodejs.org                                    https=000  (no Node provisioning; box has v22.22.3)
http://pypi.org/simple/               http=403 (port 80)
http://deb.debian.org/.../InRelease   "Connection failed [IP: 151.101.2.132 80]" -> apt unusable
tcp/22 github.com                     OPEN
```

</details>

---

*Authored inside the sandbox on 2026-09-18 (UTC). Companion file: [the research log](/research/index.md).*
