# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
and this project adheres to
[Conventional Commits](https://www.conventionalcommits.org/).

### [v0.4.1](https://github.com/Archont561/pixi-sandbox/compare/v0.4.0...19f6e36b8b6298d5867018e7cf50a0c8740269bb) (2026-10-02)

#### Features

* **ci:** move airlock gate and restore cargo wiring
([f0eba3c](https://github.com/Archont561/pixi-sandbox/commit/f0eba3cc5ecd16100c017620fb877c50f59f8986)),
closes [#55](https://github.com/Archont561/pixi-sandbox/issues/55)
* **ci:** dogfood the generated sandbox publisher on this repository
([ef9e68a](https://github.com/Archont561/pixi-sandbox/commit/ef9e68a56a588cf2007e38d7a687204197e930da))

#### Fixes

* **publish:** build sandbox transport from source
([19f6e36](https://github.com/Archont561/pixi-sandbox/commit/19f6e36b8b6298d5867018e7cf50a0c8740269bb))
* **init:** preserve pixi.toml comments (#58)
([a75005b](https://github.com/Archont561/pixi-sandbox/commit/a75005b1572bc85dd99558a9f9c72eeb223af921)),
closes [#58](https://github.com/Archont561/pixi-sandbox/issues/58)

#### Documentation

* **context:** record task-38 landing and the two open AC-clauses calls (#60)
([6beb6c3](https://github.com/Archont561/pixi-sandbox/commit/6beb6c36e90cf3690a392d475473f7044b826f5f)),
closes [#60](https://github.com/Archont561/pixi-sandbox/issues/60)
* **backlog:** record rstest airlock proof
([7920002](https://github.com/Archont561/pixi-sandbox/commit/79200026d30647a3f54a8c3f326cc796d39de75f))
* **context:** close the v0.4.0 session and correct the session skill
([540e838](https://github.com/Archont561/pixi-sandbox/commit/540e8386db2c369126b79290a8b36aa923a2e1f3)),
closes [#6](https://github.com/Archont561/pixi-sandbox/issues/6)

## [v0.4.0](https://github.com/Archont561/pixi-sandbox/compare/v0.3.7...v0.4.0) (2026-10-01)

### Features

* **ci:** generate the relock lane and commit this repository's render (#52)
([b2e028f](https://github.com/Archont561/pixi-sandbox/commit/b2e028f4cdcf49b0f66cf49e5928a4313214914e)),
closes [#52](https://github.com/Archont561/pixi-sandbox/issues/52)
* **ci:** move workflow shell into xtask and enforce the one-line step rule (#51)
([be83194](https://github.com/Archont561/pixi-sandbox/commit/be831948f74bfd395a214a2aae67a3fe7768d919)),
closes [#51](https://github.com/Archont561/pixi-sandbox/issues/51)
[#2](https://github.com/Archont561/pixi-sandbox/issues/2)
[#2](https://github.com/Archont561/pixi-sandbox/issues/2)
[#6](https://github.com/Archont561/pixi-sandbox/issues/6)
[#4](https://github.com/Archont561/pixi-sandbox/issues/4)
[#6](https://github.com/Archont561/pixi-sandbox/issues/6)
[#51](https://github.com/Archont561/pixi-sandbox/issues/51)
[#6](https://github.com/Archont561/pixi-sandbox/issues/6)
* **restore:** persist verified pixi tools on the user PATH (#48)
([5aa6ecd](https://github.com/Archont561/pixi-sandbox/commit/5aa6ecd091fe64725851bcf65a7b3df534eabac5)),
closes [#48](https://github.com/Archont561/pixi-sandbox/issues/48)
* **actions:** retire composite publishing interfaces (#47)
([5a0f0c1](https://github.com/Archont561/pixi-sandbox/commit/5a0f0c1b7a6a703907976b7f2782834bdb82a66e)),
closes [#47](https://github.com/Archont561/pixi-sandbox/issues/47)

### Fixes

* **core:** replace a running tool by staging and renaming, not copying onto it
([50c03b1](https://github.com/Archont561/pixi-sandbox/commit/50c03b180b0ce691dbf84537493d52f114de5d9a))
* **restore:** report the user-tool registration that happened, not the one requested
([fb6bc8a](https://github.com/Archont561/pixi-sandbox/commit/fb6bc8a8824fd41bce99fc2364bc90c4f352bdb0))

### Refactoring

* **tasks:** front xtask with one task and collapse the pixi task table
([fb1f47c](https://github.com/Archont561/pixi-sandbox/commit/fb1f47c84d0a232df3e14a1816b17fe6c2a91028))

### Documentation

* **context:** record the relock session and the next session's starting prompt (#53)
([99c3e17](https://github.com/Archont561/pixi-sandbox/commit/99c3e178d53e8452087f3a89329d1a310a11238a)),
closes [#53](https://github.com/Archont561/pixi-sandbox/issues/53)
* **context:** record the session close and the next session's starting point
([4b18be9](https://github.com/Archont561/pixi-sandbox/commit/4b18be9b9b0d67ca5604d62d7f538781a649f65c))
* **backlog:** add task-38 for an rstest/proptest test refactor, and draft the relock bot
([6493df9](https://github.com/Archont561/pixi-sandbox/commit/6493df92226e6c4069760671a1ca2aafe69ecf37))
* **ci:** state the workflow step rule and plan the shell migration
([3fd938b](https://github.com/Archont561/pixi-sandbox/commit/3fd938bbb1639027df5e0bd345e4dc35a3be23f5))
* **skills:** drop env sourcing from the session skill and plan the gate move
([d1af2fe](https://github.com/Archont561/pixi-sandbox/commit/d1af2fe878a2e8a4f649ec8b79684d82e3e8f4b9))
* **backlog:** close release package task (#49)
([8e2b99f](https://github.com/Archont561/pixi-sandbox/commit/8e2b99f065cf111b50efba86fcbc9951bb17508c)),
closes [#49](https://github.com/Archont561/pixi-sandbox/issues/49)

### [v0.3.7](https://github.com/Archont561/pixi-sandbox/compare/v0.3.6...v0.3.7) (2026-10-01)

#### Fixes

* **release:** replace repo-targeting shell scripts with tested xtask commands (#46)
([910df8f](https://github.com/Archont561/pixi-sandbox/commit/910df8f2ff39412d62df82ac81563e17ffd45f92)),
closes [#46](https://github.com/Archont561/pixi-sandbox/issues/46)

### [v0.3.6](https://github.com/Archont561/pixi-sandbox/compare/v0.3.5...v0.3.6) (2026-10-01)

#### Features

* generate provider-neutral direct publish workflows (#45)
([279d748](https://github.com/Archont561/pixi-sandbox/commit/279d748cfd3ade87badf0d81645803fd274e3b47)),
closes [#45](https://github.com/Archont561/pixi-sandbox/issues/45)
* publish conda packages per platform and keep bun out of the default env
([cdf60b7](https://github.com/Archont561/pixi-sandbox/commit/cdf60b796f2155db650363dacf031da9605c5e81))

#### Fixes

* **dev:** install the JS workspace before linting it
([3f28c26](https://github.com/Archont561/pixi-sandbox/commit/3f28c26766659ea4e7cf4046644bd47103b5f3c6))

#### Documentation

* organize v1 planning and network boundaries
([375577c](https://github.com/Archont561/pixi-sandbox/commit/375577c75335c17684a085f4bdad88cbce4ce4f0))

### [v0.3.5](https://github.com/Archont561/pixi-sandbox/compare/v0.3.4...v0.3.5) (2026-10-01)

#### Fixes

* **release:** use canonical prefix channel
([0a91159](https://github.com/Archont561/pixi-sandbox/commit/0a91159a51a93cff53272b51c6dbd38a14fbfac9))

### [v0.3.4](https://github.com/Archont561/pixi-sandbox/compare/v0.3.3...v0.3.4) (2026-10-01)

#### Fixes

* **release:** pass absolute build/target dirs to pixi publish
([b80fe41](https://github.com/Archont561/pixi-sandbox/commit/b80fe417e84fb0a1ecaa3616917d5847501571fd))

### [v0.3.3](https://github.com/Archont561/pixi-sandbox/compare/v0.3.2...v0.3.3) (2026-09-30)

#### Features

* **release:** publish pixi-sandbox package to prefix.dev
([2575347](https://github.com/Archont561/pixi-sandbox/commit/257534712d889d52b2e5a0026acf28339931659c))

#### Refactoring

* **devcontainer:** move postCreateCommand into setup.sh
([535b0fc](https://github.com/Archont561/pixi-sandbox/commit/535b0fc11bbaf6dd553be2ec6d7c614ce5318398))

#### Documentation

* **release:** describe prefix package in autorelease
([860b036](https://github.com/Archont561/pixi-sandbox/commit/860b0366792848f8cfdb531228afa9ec2b4b3d25))

#### Build System

* derive the docs version from Cargo.toml instead of repinning literals (#39)
([9b0150d](https://github.com/Archont561/pixi-sandbox/commit/9b0150d1bf8c8d1518e2f3bd169d44c720156feb)),
closes [#39](https://github.com/Archont561/pixi-sandbox/issues/39)

### [v0.3.2](https://github.com/Archont561/pixi-sandbox/compare/v0.3.1...v0.3.2) (2026-09-29)

#### Fixes

* make the generated publish workflow and the published action paths runnable (#38)
([f157a6d](https://github.com/Archont561/pixi-sandbox/commit/f157a6d51180808d0a8c3921e09d6a12896f7859)),
closes [#38](https://github.com/Archont561/pixi-sandbox/issues/38)
[#37](https://github.com/Archont561/pixi-sandbox/issues/37)

#### CI

* drop osx-arm64 from routine publish plan and refresh docs on release (#36)
([26e1349](https://github.com/Archont561/pixi-sandbox/commit/26e13491ea341c198981e55ecf86b8210e8c93f9)),
closes [#36](https://github.com/Archont561/pixi-sandbox/issues/36)

### [v0.3.1](https://github.com/Archont561/pixi-sandbox/compare/v0.3.0...v0.3.1) (2026-09-29)

#### Features

* **sandbox:** declare the osx-arm64 developer bundle to schedule the D11 proof
([78b60cb](https://github.com/Archont561/pixi-sandbox/commit/78b60cb0e35f671da4d07a4c771cb38e2cb02c08))
* **core:** verify a restored project against the manifest, not just its shape (task-10, D13)
([30272a9](https://github.com/Archont561/pixi-sandbox/commit/30272a98f1cb6d39687794d66348c0e2b2255660))
* **tools:** implement tools update, with honest verification (task-4)
([fdabd2b](https://github.com/Archont561/pixi-sandbox/commit/fdabd2b7b10d7892cce93639822c8d91c49feb88))

#### Fixes

* **release:** drop the old-tag argument from the release repin
([bdebeaa](https://github.com/Archont561/pixi-sandbox/commit/bdebeaa6dfab62b4eaf72393d53feac6a6d4f610))
* **release:** make the version references data driven
([305764a](https://github.com/Archont561/pixi-sandbox/commit/305764a87921e6b061a1f3cd5ba24646be5ffaf0))
* **ci:** publish sandboxes only from the default branch
([451e0b5](https://github.com/Archont561/pixi-sandbox/commit/451e0b5601b8007e6b1cae43f60d134353199510))
* **airlock:** stop one block of allocator noise failing the no-op gate
([2c612f9](https://github.com/Archont561/pixi-sandbox/commit/2c612f958a0b473d1585894112a731808019f0b8))
* **airlock:** run the egress-denied tier on the triggers that actually fire
([8e03cdf](https://github.com/Archont561/pixi-sandbox/commit/8e03cdfd6dabdb21cc2098b76d78b82a25767a72))
* **setup:** stop a rate-limited releases API from failing silently
([10fa729](https://github.com/Archont561/pixi-sandbox/commit/10fa729ff8c77be03d065932c7e65b6f41ccf4e4))
* **setup:** install the release binary on macOS, where bash is still 3.2
([26eac11](https://github.com/Archont561/pixi-sandbox/commit/26eac1162bae904c54708db2f7b3f81180f7d25e))
* **gate:** pixi install runs inside the restored project, not the caller's cwd
([4c1c4a1](https://github.com/Archont561/pixi-sandbox/commit/4c1c4a1aae0a0cd92195eb1e6343de2f09930e45))
* **ci:** proof job needs pixi on PATH and the root-action subpath spelling
([e1ed6a6](https://github.com/Archont561/pixi-sandbox/commit/e1ed6a6af85512a193277a956e5fcbae1d713edb)),
closes [#16](https://github.com/Archont561/pixi-sandbox/issues/16)
* **ci:** the airlock workflow's first run — plan job cwd, local action paths, gate release
transition
([ea3339a](https://github.com/Archont561/pixi-sandbox/commit/ea3339a974cf01eb27feb896cdeed4a6dd2f48c0)),
closes [#16](https://github.com/Archont561/pixi-sandbox/issues/16)
* **pack:** refuse a lockfile cargo cannot vendor (task-8)
([7dcfd08](https://github.com/Archont561/pixi-sandbox/commit/7dcfd08dbe60f7b042aaebe4afa99b5ed167b6cc))
* **release:** stamp the tag into the published init.sh
([2ac6284](https://github.com/Archont561/pixi-sandbox/commit/2ac6284e498c48826e30b6ac3e7319c4b2f726cb))
* **docs:** point the install one-liner at the v0.3.0 release (task-2)
([265a420](https://github.com/Archont561/pixi-sandbox/commit/265a420e2340bd0ee65e36fd68a2481752c5f349))

#### Refactoring

* **plan:** make the plan binary the only platform-to-runner table
([5572b2c](https://github.com/Archont561/pixi-sandbox/commit/5572b2c50162093892395d2b505271f832bf2063))

#### Documentation

* **readme:** mark osx-arm64 proven on a native runner with egress denied
([5f615b1](https://github.com/Archont561/pixi-sandbox/commit/5f615b13d42a59849b48dd584b22ebda00b0095a))
* repin every v0.2.0 reference to the v0.3.0 release
([f43963c](https://github.com/Archont561/pixi-sandbox/commit/f43963c9c71a55ce00d9051025baa1cfdd223a51))

#### CI

* pin every action to a commit SHA, and lint that it stays that way
([5c85bc2](https://github.com/Archont561/pixi-sandbox/commit/5c85bc2bee6ab2030dc4acdb5690891ba5edd651))
* prove the airlock natively, with egress actually denied
([ec73095](https://github.com/Archont561/pixi-sandbox/commit/ec7309559fd4b65e0bcd9cdf02620576927306b7))
* fail when the docs pin a version the manifests do not declare
([0a9e18d](https://github.com/Archont561/pixi-sandbox/commit/0a9e18d84eeb302dc1c6cb2953d87adef140e606))

## [v0.3.0](https://github.com/Archont561/pixi-sandbox/compare/v0.2.0...v0.3.0) (2026-09-29)

### Features

* **publish:** rotate the branch with --keep N
([4e4ffa6](https://github.com/Archont561/pixi-sandbox/commit/4e4ffa68123f1d783a600bcc2ded099df7d09ae9))
* **skills:** add local backlog.md workflow skill
([bae5e60](https://github.com/Archont561/pixi-sandbox/commit/bae5e6091451a6db0ccf958bb3c2386a6c282f2f))
* **ci:** add dry-run mode to publish-sandbox
([0844fb7](https://github.com/Archont561/pixi-sandbox/commit/0844fb73cdd35a28ab2d94ae701e46f6c0291626))
* **init:** derive the launcher branch from the reviewed plan
([2a1a0a1](https://github.com/Archont561/pixi-sandbox/commit/2a1a0a17a068f3fae5c887efd1d5c46ce42d7b5e))
* **restore:** derive the sandbox branch from .pixi-sandbox.toml
([0ba40d7](https://github.com/Archont561/pixi-sandbox/commit/0ba40d7f7bcd039679ebc0007490972a17d21aef))
* pin generated actions to latest commit
([60daa44](https://github.com/Archont561/pixi-sandbox/commit/60daa44622e78ef00582ed7d8761d01f5a4517dc))
* generate project-side airlock bootstrap
([4df9a79](https://github.com/Archont561/pixi-sandbox/commit/4df9a79e21be36f9d3a672958d019ebb55969777))

### Fixes

* **cli:** cite only design.md for deferred verbs
([2520562](https://github.com/Archont561/pixi-sandbox/commit/252056207a285a5e72c6fb43328fd9373876abcb))
* **restore:** delete the staging scratch once an environment is in place
([fca1f0e](https://github.com/Archont561/pixi-sandbox/commit/fca1f0eb3a0d270ed02258aa06528ed1ec78ebfe)),
closes [#18](https://github.com/Archont561/pixi-sandbox/issues/18)
[#18](https://github.com/Archont561/pixi-sandbox/issues/18)
* remove orphaned node dependencies from lockfile
([cedf273](https://github.com/Archont561/pixi-sandbox/commit/cedf273a096105194e18d341594f533ad9172ed9))
* relocate restored environment text prefixes
([11beaf9](https://github.com/Archont561/pixi-sandbox/commit/11beaf94cfe825651c1bd19083b4864899bb9955))
* **ci:** build static sandbox bootstrap
([68cda7e](https://github.com/Archont561/pixi-sandbox/commit/68cda7ea46fc3433af07c06b18f1de00395f4fed))
* **ci:** point local action uses at directories, not action.yml files (#21)
([7e3b012](https://github.com/Archont561/pixi-sandbox/commit/7e3b0123897cd1678f3eadf5b1dd193a548dab95)),
closes [#21](https://github.com/Archont561/pixi-sandbox/issues/21)
* **ci:** build the publish matrix from plan --json `.include` only
([e1b3755](https://github.com/Archont561/pixi-sandbox/commit/e1b3755b8e532b2f0e9502721a73ec19dec65472)),
closes [#16](https://github.com/Archont561/pixi-sandbox/issues/16)
* **ci:** point the publish plan's setup-pixi steps at project/ (#16)
([bff760e](https://github.com/Archont561/pixi-sandbox/commit/bff760e2feef46ed625bb0056f6ef8362956ba24)),
closes [#16](https://github.com/Archont561/pixi-sandbox/issues/16)
* **ci:** track docs with the bun ecosystem so frozen installs stay valid
([bebac6d](https://github.com/Archont561/pixi-sandbox/commit/bebac6d4db6984c1d8ce10d0e73c09b7b3201106))

### Refactoring

* single pixi environment, linux/macos-only, one root action
([809f934](https://github.com/Archont561/pixi-sandbox/commit/809f934bbd36e5c50a13d9b51b9bf3b3a54e547d))

### Documentation

* sync the README devcontainer quote with the actual command
([8d6cca6](https://github.com/Archont561/pixi-sandbox/commit/8d6cca6a87171e507f84e353244f28515a173c02))
* fix install one-liner (task-2), record rattler no-go D12 (task-9)
([6fd1152](https://github.com/Archont561/pixi-sandbox/commit/6fd1152c992ced5933b72fb5e5fd7ef7adbb7ece)),
closes [#1](https://github.com/Archont561/pixi-sandbox/issues/1)
[#3](https://github.com/Archont561/pixi-sandbox/issues/3)
* **knowledge:** measure the rattler v2 trade-off (task-9 spike)
([edf94e1](https://github.com/Archont561/pixi-sandbox/commit/edf94e16468b90468b72c57f9db8585ea80d7c84)),
closes [#2](https://github.com/Archont561/pixi-sandbox/issues/2)
* **repository:** fix the dangling sentence left by the prototype removal
([33e6076](https://github.com/Archont561/pixi-sandbox/commit/33e607618393ca3452a09a042275851396bb7d81))
* **context:** a restored environment names itself, not the scratch
([35066b5](https://github.com/Archont561/pixi-sandbox/commit/35066b59445ca02029165e60240fb9414225a98e))
* **readme:** resync with the code and provision git/gh in the container
([244b51e](https://github.com/Archont561/pixi-sandbox/commit/244b51e377185c43e54b9fa954aa9e0738d3bf5e))
* reduce setup and restore to two commands
([8c4ec2b](https://github.com/Archont561/pixi-sandbox/commit/8c4ec2bc79834927b50fb5232ca2d4e035424189))

### Build System

* **devcontainer:** provision a C compiler, so the Rust gates can run at all
([42ff52c](https://github.com/Archont561/pixi-sandbox/commit/42ff52c099f15f5d4b4b35b5918e24a3eedbb8ae))
* **dev:** one bun workspace at the root, with the repo dev tooling in it
([b35eddb](https://github.com/Archont561/pixi-sandbox/commit/b35eddb1788f88c8cf4076e19f5352baea1d7d4b))
* **devcontainer:** provision the agent CLI in the container
([8ef9c81](https://github.com/Archont561/pixi-sandbox/commit/8ef9c81c837b936039ab5674dd058009f8af3f52))
* **devcontainer:** use the official pixi image directly
([e581c79](https://github.com/Archont561/pixi-sandbox/commit/e581c79167cbae5fbf2ca20faa374aac1d75d794))
* add Pixi development container
([4a15985](https://github.com/Archont561/pixi-sandbox/commit/4a15985c0c81ec7a9cd045cb3dc9712940b6adbb))
* remove Node.js from development environment
([3ad1f01](https://github.com/Archont561/pixi-sandbox/commit/3ad1f01b9b836d927d00c505de0e20f73ccdbf95))

### CI

* publish scripts/init.sh as a release asset (task-2)
([5a765eb](https://github.com/Archont561/pixi-sandbox/commit/5a765ebc566531d4f1235218aaa1fcd549ed7d18))
* add on-demand auto-release workflow
([5fd7b3f](https://github.com/Archont561/pixi-sandbox/commit/5fd7b3f4625c3b41aadbde52e99e54d8faaae66a))

## v0.2.0 (2026-09-21)

### Features

* self-bootstrap at branch root and apply dependency updates
([72d466a](https://github.com/Archont561/pixi-sandbox/commit/72d466a27d0dc79d66ace6ef0360e12c4f93c905))
* short refs for setup and publish in same repo
([6434700](https://github.com/Archont561/pixi-sandbox/commit/6434700dc1d93e2b337f579dd7881ad6cc07f74f))
* one-liner offline reconstruction with PATH aliases
([81813e1](https://github.com/Archont561/pixi-sandbox/commit/81813e14536f4e69e318c261d8615114816fdbcb))
* make setup action usable like prefix-dev/setup-pixi
([2bec5d7](https://github.com/Archont561/pixi-sandbox/commit/2bec5d7ecd51f2111589e5befa704262a4506cc7))

### Fixes

* biome lint-docs failures (formatting, SVG a11y, Astro false positives)
([901095a](https://github.com/Archont561/pixi-sandbox/commit/901095a9fae8fcfe561a04f75949f1454781fad4))
* unify publish workflows, rename output-path, add release workflow, remove python refs, fix
scratch leak
([0c90180](https://github.com/Archont561/pixi-sandbox/commit/0c9018060ce0c80e8693e75f3a08256eb7ef73eb))
* **e2e:** simplify unshare_check with is_ok_and for clippy
([0e3eb86](https://github.com/Archont561/pixi-sandbox/commit/0e3eb86867a93bcb948768569cb8d45c0a05ec6e))
* **ci:** add Cargo caching and consolidate CI into unified single job
([91b6848](https://github.com/Archont561/pixi-sandbox/commit/91b6848cc048385a49c234973e8dce63ecbfe91e))
* **ci:** resolve CI failures and rewrite action and test logic in Rust
([0379c4f](https://github.com/Archont561/pixi-sandbox/commit/0379c4f96cca0ad443b739fdb9d625ac557f54b2))

### Documentation

* regenerate README with badges, sandbox viz, guides, config ref
([1b50d03](https://github.com/Archont561/pixi-sandbox/commit/1b50d034fce0ab96ab3cf07e8b363814f7f16ae8))
* redesign to astro-icon + iconify with sandbox visualization
([d170922](https://github.com/Archont561/pixi-sandbox/commit/d1709220fa3c8bf0ff38f7d0919d87ff1522b392))
* add CHANGELOG.md via convco and changelog tasks
([b008ad7](https://github.com/Archont561/pixi-sandbox/commit/b008ad7ba9ecc33b402891ef7f9c2e7e7ab7f9ce))
* clean remaining python refs in fixtures and evidence
([3c19540](https://github.com/Archont561/pixi-sandbox/commit/3c19540409b9fe8e4c2ace4e1c933fb52b427482))
* remove all python references, update to pure Rust bootstrap
([c748d76](https://github.com/Archont561/pixi-sandbox/commit/c748d76b80024fa0d7114c531b21ef624a26967f))
* regenerate READMEs and CONTEXT with shields, badges, and enhanced markdown
([eb25794](https://github.com/Archont561/pixi-sandbox/commit/eb25794673f9ddc2a6c6fc2fd66970d75bf3560f))

### CI

* **deps:** bump actions/upload-pages-artifact from 3.0.1 to 5.0.0
([a2409bf](https://github.com/Archont561/pixi-sandbox/commit/a2409bf5ceec6d08a56f7fa304e8d29ba08be834))
* **deps:** bump actions/deploy-pages from 4.0.5 to 5.0.1
([2c48646](https://github.com/Archont561/pixi-sandbox/commit/2c486469629a9777a86a5d23ce8217b418e2e066))
* **deps:** bump actions/configure-pages from 5.0.0 to 6.0.0
([a3d7a1e](https://github.com/Archont561/pixi-sandbox/commit/a3d7a1ec974def61cc5194cf41a980af550a6922))
* **deps:** bump softprops/action-gh-release from 2.4.1 to 3.0.3
([85a5fbd](https://github.com/Archont561/pixi-sandbox/commit/85a5fbd5d86a6b79c28fb7c644675d7c400147b6))
* **deps:** bump actions/download-artifact from 7.0.1 to 8.0.1
([5ab22cd](https://github.com/Archont561/pixi-sandbox/commit/5ab22cdf20e1a61dd16f9691a30cf58aeacf7b5f))
* **deps:** bump codecov/codecov-action from 5.4.0 to 7.1.1
([1a30e00](https://github.com/Archont561/pixi-sandbox/commit/1a30e00c636e466e209837641ed78bdf52541dda))
* add dependabot for cargo, gha, and npm (docs)
([5733af2](https://github.com/Archont561/pixi-sandbox/commit/5733af2ab323dfe52c8bcaeb28237c720606c72c))
* upgrade workflow actions to Node 24 native versions
([9be64a3](https://github.com/Archont561/pixi-sandbox/commit/9be64a3ad88131c81d83f8baf7baf06f9089620a))
* add codecov configuration and status badge
([d4956b3](https://github.com/Archont561/pixi-sandbox/commit/d4956b3694ac82d752ec682641f87d895e63540d))
* trigger publish-sandbox after ci is green with workflow_run and workflow_dispatch
([be50f7b](https://github.com/Archont561/pixi-sandbox/commit/be50f7bcdc16d2b78b61bd63f3f1b92d54d856b4))
