# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
and this project adheres to
[Conventional Commits](https://www.conventionalcommits.org/).

## [v0.9.0](https://github.com/Archont561/pixi-sandbox/compare/v0.8.0...f56123741733b239b929f6e59f3e22c6657c76b9) (2026-10-25)

### Features

* **plan:** report per-workspace vendor costs and shared-crate overlap (TASK-131 slice 4)
([a11b263](https://github.com/Archont561/pixi-sandbox/commit/a11b263545576864876e9cb4d9bd50f1bc0bf741))
* **restore:** materialise per-workspace vendor trees with per-project cargo wiring (TASK-131
slice 3)
([b6ed1a5](https://github.com/Archont561/pixi-sandbox/commit/b6ed1a55ba3eb6fcf90986c6ff46a5b369c936e1))
* **pack:** vendor one cargo tree per workspace into qualified transport dirs (TASK-131 slice 2)
([572141a](https://github.com/Archont561/pixi-sandbox/commit/572141ae3029f4ba4102a017b28128a1cae3ea77))
* **manifest:** schema 5 workspace_vendors for per-workspace cargo trees (TASK-131 slice 1)
([2c7c5a7](https://github.com/Archont561/pixi-sandbox/commit/2c7c5a7deaadc8ca406d13d0af0fc93b9e23c87b)),
closes [#1](https://github.com/Archont561/pixi-sandbox/issues/1)
* **plan:** name nested-workspace channels and resolved package identities
([a66a8eb](https://github.com/Archont561/pixi-sandbox/commit/a66a8eb70ad7f54759ddbd5312995f919fd95cbd)),
closes [#3](https://github.com/Archont561/pixi-sandbox/issues/3)
* **xtask:** start the auto-release lane from a milestone-v* tag
([bd0559a](https://github.com/Archont561/pixi-sandbox/commit/bd0559aa3d4cf02c8e061643e49fad0ac9b2cd66))
* **workspaces:** bundle explicitly declared nested Pixi workspaces
([9cc5b55](https://github.com/Archont561/pixi-sandbox/commit/9cc5b558fad7407e230675af833f4e73465ba0d5)),
closes [#3](https://github.com/Archont561/pixi-sandbox/issues/3)
[#7](https://github.com/Archont561/pixi-sandbox/issues/7)

### Fixes

* **test:** glob vendor transport entries instead of reading prefix paths (TASK-131)
([fd794ce](https://github.com/Archont561/pixi-sandbox/commit/fd794cefb5a9549085bf0d181c2b466392a23b77))
* **airlock:** empty the filter token so the push range is honored locally
([34ffc42](https://github.com/Archont561/pixi-sandbox/commit/34ffc421bc9a729da496f6afc01dd37156609a09)),
closes [#207](https://github.com/Archont561/pixi-sandbox/issues/207)
* **airlock:** use some-semantics in the per-push filter so mixed pushes prove
([d3bafe5](https://github.com/Archont561/pixi-sandbox/commit/d3bafe54627b4f530bf826aa9408d8b38dbeb16b)),
closes [#207](https://github.com/Archont561/pixi-sandbox/issues/207)
* **ci:** restore into a checkout, not a fat clone with a moved .pixi
([7223fc6](https://github.com/Archont561/pixi-sandbox/commit/7223fc65f73611b738a17d8e25592c53ebb67c07))
* **ci:** require pixi-unpack only from legacy airlock transports
([b94330a](https://github.com/Archont561/pixi-sandbox/commit/b94330ad3d9a21d88b9fce4358d8bdd9493107cc))
* **deps:** update ctutils and tokio for the two advisories
([8a3d35b](https://github.com/Archont561/pixi-sandbox/commit/8a3d35b510d753f79c438de5543b044a407759a6))
* **pack:** accept bare absolute paths and http mirrors in locked artifacts
([94a6e68](https://github.com/Archont561/pixi-sandbox/commit/94a6e684ce9a631f9280ceb16aeea4d72557b0e1))

### Documentation

* **backlog:** close TASK-135, add TASK-136 release gate and m-1 criterion-3 ruling
([c2644bd](https://github.com/Archont561/pixi-sandbox/commit/c2644bd092af6036458a6e988e7d4e42353f686f))
* **task-129:** close with the AC#7 native evidence and full run ledger
([9695e65](https://github.com/Archont561/pixi-sandbox/commit/9695e654ffcd5e12705b04066d6c1a41f935a5bc)),
closes [#7](https://github.com/Archont561/pixi-sandbox/issues/7)
[#7](https://github.com/Archont561/pixi-sandbox/issues/7)
* **task-129:** reconstruct Stage A spike evidence and record the AC#3 rescope ruling
([949db43](https://github.com/Archont561/pixi-sandbox/commit/949db43cec5e5ecea7ed759e3383f10e60599157)),
closes [#3](https://github.com/Archont561/pixi-sandbox/issues/3)
[#3](https://github.com/Archont561/pixi-sandbox/issues/3)
[#7](https://github.com/Archont561/pixi-sandbox/issues/7)
* **backlog:** attach m-1 to TASK-129/131 and close the finished TASK-125
([4b97348](https://github.com/Archont561/pixi-sandbox/commit/4b97348a5635b9298420d0db401403cffa2a326a))
* **backlog:** attach the TASK-135 evidence ledger and name the open native-runner clauses
([c286f2b](https://github.com/Archont561/pixi-sandbox/commit/c286f2ba7a8a19ef445590d529ffa6cd597685f4))
* **backlog:** root-cause the second TASK-135 filter fault to the action token
([a6630f5](https://github.com/Archont561/pixi-sandbox/commit/a6630f501dd948ce25d94a3fb5b832e34d47ef80))
* **backlog:** record TASK-135's first real-runner evidence on PR #207
([d087805](https://github.com/Archont561/pixi-sandbox/commit/d087805a2006fee52164db811978f625fdb8321a)),
closes [#207](https://github.com/Archont561/pixi-sandbox/issues/207)
* **session:** hand off the next session in chat instead of a follow-up PR (TASK-135)
([01f4329](https://github.com/Archont561/pixi-sandbox/commit/01f43293b4860f6f50f8dbd7969cc14040c45a54)),
closes [#206](https://github.com/Archont561/pixi-sandbox/issues/206)
* **sprint:** make a session a sprint and open milestone m-1 (0.9.0)
([bf862fe](https://github.com/Archont561/pixi-sandbox/commit/bf862fefd49ad5c5fdae8c6387ab0fe1b3e721a2))
* **backlog:** record the two TASK-134 causes and the green airlock proof
([47df2bc](https://github.com/Archont561/pixi-sandbox/commit/47df2bc39af892b339490f526e1e4e34fa6f265e))
* **verification:** call the oracle by one name and the manifest by its schema
([70f9d08](https://github.com/Archont561/pixi-sandbox/commit/70f9d0881de49a011693ba1805e5743e60456396))
* **backlog:** file TASK-134 for the pre-existing airlock Tier B failure
([7fd8861](https://github.com/Archont561/pixi-sandbox/commit/7fd88615e80237a50c226aeecc869a86f0ef431d))
* **backlog:** close TASK-133 and record the two-phase verification
([bf3f859](https://github.com/Archont561/pixi-sandbox/commit/bf3f8597862104d929aea3894f8411ef3d69090e))
* **backlog:** file the ctutils and tokio advisory update
([33fd426](https://github.com/Archont561/pixi-sandbox/commit/33fd426aa4a0afedd93f4586408a280a6b82b241))
* **backlog:** record TASK-129 slice 0 premise evidence and file TASK-132
([c6bcaa2](https://github.com/Archont561/pixi-sandbox/commit/c6bcaa27c27c04dc0d5709107a28532b304096cf))
* **backlog:** ratify TASK-129 design decisions and split out per-workspace vendoring
([8b3f2c2](https://github.com/Archont561/pixi-sandbox/commit/8b3f2c2a011cf12d0105f59be071ea3f61686974))
* **backlog:** record nested-workspace transport design and branch topology decisions
([ca1a8fb](https://github.com/Archont561/pixi-sandbox/commit/ca1a8fb43601012de73b0d568be484fc7129eccd))

### CI

* per-push airlock skip plus an always-restored rust cache for docs PRs (TASK-135)
([9ce4933](https://github.com/Archont561/pixi-sandbox/commit/9ce493311fca52125f853b5ba610091360da6111)),
closes [#202](https://github.com/Archont561/pixi-sandbox/issues/202)
[#206](https://github.com/Archont561/pixi-sandbox/issues/206)

## [v0.8.0](https://github.com/Archont561/pixi-sandbox/compare/v0.7.0...v0.8.0) (2026-10-22)

### ⚠ BREAKING CHANGE

* Document the native migration's removed tools command and
helper flags, explicit Pixi/bootstrap bundling, schema-3 output, v7 locks
and Rust 1.94 source minimum. Recommend v0.8.0 under the pre-1.0 minor policy;
do not bump manifests, dispatch production or create a release in this change.



### Features

* **transport:** pack and restore native format-1 environments
([a858596](https://github.com/Archont561/pixi-sandbox/commit/a85859680de040b539593c72b2fc36c95b7f6619))
* install bare packs with native rattler and wheel libraries
([e502031](https://github.com/Archont561/pixi-sandbox/commit/e5020310da52a7bb7a1a28194000845efeb1a093))
* **deps:** add rattler 0.48 (rustls) to replace pixi-pack/unpack – TASK-125
([aac323f](https://github.com/Archont561/pixi-sandbox/commit/aac323f1109f9c42819deaf48e8d603b2e7ed642))
* **backlog:** add TASK-125 rattler migration abandon tools and TASK-126 CI optimization follow-up
([f1b8032](https://github.com/Archont561/pixi-sandbox/commit/f1b8032243796bedc9957a57e7a0e588b9da5790))
* **self-update:** adopt self_update crate transport per ADR-27 (TASK-124)
([260030d](https://github.com/Archont561/pixi-sandbox/commit/260030d8d15ce2de501b3fd2dfd2b2f54faacc31))
* **backlog:** add TASK-124 adopt self_update transport per ADR-27
([537fa50](https://github.com/Archont561/pixi-sandbox/commit/537fa50d721c0ce44b3698d8249e1292c378c534))
* **relock:** add path filtering to reduce PR minutes (TASK-121, TASK-122)
([fcd682d](https://github.com/Archont561/pixi-sandbox/commit/fcd682d8fca8e81601554da45b0779fc4185add0)),
closes [#179](https://github.com/Archont561/pixi-sandbox/issues/179)
[#179](https://github.com/Archont561/pixi-sandbox/issues/179)
* **workflow:** add default concurrency to generated publish workflow
([2c23d51](https://github.com/Archont561/pixi-sandbox/commit/2c23d5176bfc001e87c0094068eadaa905ec3623)),
closes [#179](https://github.com/Archont561/pixi-sandbox/issues/179)

### Fixes

* decode compact locks and streamline CI builds
([4a5955c](https://github.com/Archont561/pixi-sandbox/commit/4a5955c3a9a63f2288b9e6f2cec8b9f23852467d))
* **ci:** remove unused rattler and refresh vulnerable TLS pins (#186)
([71be6d5](https://github.com/Archont561/pixi-sandbox/commit/71be6d5deb9b63f07f290c81eae0c089094417be)),
closes [#186](https://github.com/Archont561/pixi-sandbox/issues/186)
* **xtask:** align starter failure priority with workflow order (#175)
([84e8010](https://github.com/Archont561/pixi-sandbox/commit/84e8010a5dd9e8408a4f94da33c111a05244b23e)),
closes [#175](https://github.com/Archont561/pixi-sandbox/issues/175)
* **files-manifest:** collapse the installer's prefix-stamped script rows (#173)
([897964d](https://github.com/Archont561/pixi-sandbox/commit/897964d28d7a69b86c97ed528d2a501320c5f07e)),
closes [#173](https://github.com/Archont561/pixi-sandbox/issues/173)
[#170](https://github.com/Archont561/pixi-sandbox/issues/170)
[#170](https://github.com/Archont561/pixi-sandbox/issues/170)
* **starter:** refuse a missing token early and report the failed phase
([86a7ec4](https://github.com/Archont561/pixi-sandbox/commit/86a7ec48ad9a7f2df4ee416db024d4d6a31f6d30)),
closes [#101](https://github.com/Archont561/pixi-sandbox/issues/101)
[#8](https://github.com/Archont561/pixi-sandbox/issues/8)
[#10](https://github.com/Archont561/pixi-sandbox/issues/10)
[#7](https://github.com/Archont561/pixi-sandbox/issues/7)
[#8](https://github.com/Archont561/pixi-sandbox/issues/8)
[#2](https://github.com/Archont561/pixi-sandbox/issues/2)
[#9](https://github.com/Archont561/pixi-sandbox/issues/9)
[#4](https://github.com/Archont561/pixi-sandbox/issues/4)
[#10](https://github.com/Archont561/pixi-sandbox/issues/10)
* **starter:** repair publication and default to latest release (#164)
([e16884a](https://github.com/Archont561/pixi-sandbox/commit/e16884a2b8615aeb2c2de7da8b0a3abca33898bb)),
closes [#164](https://github.com/Archont561/pixi-sandbox/issues/164)

### Performance

* **workflows:** reuse trusted builds and shorten supporting jobs
([8729711](https://github.com/Archont561/pixi-sandbox/commit/8729711bd26a275a3e469cd4825b2ae2c921b86f))
* **ci:** streamline verified workflow bootstrap (TASK-126)
([dda8bea](https://github.com/Archont561/pixi-sandbox/commit/dda8bea7b6c4d40e3f25e4093378ac73a1df520c))

### Refactoring

* **prefix:** share staged-prefix promotion policy
([618e7ce](https://github.com/Archont561/pixi-sandbox/commit/618e7ce1a2a183cce4d7e65d08acad32d5a4d0e9))

### Documentation

* document native transport migration and release readiness
([fb9abb9](https://github.com/Archont561/pixi-sandbox/commit/fb9abb9d59441b48dd87a549f89e7c5c62a7220f))
* **context:** record TASK-126 verification and remaining proof
([cc997b4](https://github.com/Archont561/pixi-sandbox/commit/cc997b42ae9040eaa574784b4534a7d27b725b07))
* **backlog:** close TASK-66,74,116 with live evidence and documented exceptions
([ea7621c](https://github.com/Archont561/pixi-sandbox/commit/ea7621ca156fd9f17418a61010259c8c173ca01b)),
closes [#135](https://github.com/Archont561/pixi-sandbox/issues/135)
[#56](https://github.com/Archont561/pixi-sandbox/issues/56)
* **backlog:** mark TASK-121,122 Done with ACs checked
([b170e9a](https://github.com/Archont561/pixi-sandbox/commit/b170e9aaf7896e23a0c052a1caf2a0d3d280267f))
* **backlog:** update TASK-118,119,121 with measurements and proposals, mark 120,123 Done
([dafa073](https://github.com/Archont561/pixi-sandbox/commit/dafa073be54ee5af972003c0d6696b80089c039f))
* **workflow:** reflect default concurrency now on by default
([3fb0886](https://github.com/Archont561/pixi-sandbox/commit/3fb08861b0765e2682bf96bbfa563ee6e30a46a6))
* **backlog:** record TASK-117 investigation evidence (#172)
([abd2df3](https://github.com/Archont561/pixi-sandbox/commit/abd2df3f1345d4c3968293b0fcc65d49b64e18df)),
closes [#172](https://github.com/Archont561/pixi-sandbox/issues/172)
* **pack:** document PyPI wheel handling (#171)
([ef21890](https://github.com/Archont561/pixi-sandbox/commit/ef21890156d390bf3b54c08d7f0d7c5c14882f4f)),
closes [#171](https://github.com/Archont561/pixi-sandbox/issues/171)
* **context:** record the merge of the starter failure-reporting session
([6626180](https://github.com/Archont561/pixi-sandbox/commit/6626180ca507b17489c313da7077fd22668a1c0e)),
closes [#168](https://github.com/Archont561/pixi-sandbox/issues/168)
* **context:** record the starter failure-reporting session
([6f2c83e](https://github.com/Archont561/pixi-sandbox/commit/6f2c83e2555b728a77f716ebb9cda7fbe29fffd8))
* **backlog:** apply the TASK-74 owner decisions to the starter contract (#167)
([6184724](https://github.com/Archont561/pixi-sandbox/commit/61847243078a0f8202a6819be9a6dc328d83f9c4)),
closes [#167](https://github.com/Archont561/pixi-sandbox/issues/167)
* **context:** record the GitHub App deprecation and next prompt
([2a90b96](https://github.com/Archont561/pixi-sandbox/commit/2a90b960cb8537df042b36f5f1053e47c06f630e))
* **backlog:** deprecate TASK-73, the owner-operated GitHub App task
([2442978](https://github.com/Archont561/pixi-sandbox/commit/244297867a9c8269925f7d5c8f7e9eb3994c0cd9)),
closes [#7](https://github.com/Archont561/pixi-sandbox/issues/7)
* **context:** record the v0.7.0 starter evidence and next prompt
([cc73133](https://github.com/Archont561/pixi-sandbox/commit/cc731337bcc6c9493ac6daf3279094b240899320))
* **readme:** retire the unreplaced-run claim from the v0.6.2 starter note
([a37ba8c](https://github.com/Archont561/pixi-sandbox/commit/a37ba8c3d79e1a04d046cf9c6a500f2c110d4d01))
* **backlog:** close TASK-85, TASK-94 and TASK-95 on the v0.7.0 starter run
([32b41d7](https://github.com/Archont561/pixi-sandbox/commit/32b41d7bef4ecec3c43a28f2fa42d190982527dd)),
closes [#5](https://github.com/Archont561/pixi-sandbox/issues/5)
[#4](https://github.com/Archont561/pixi-sandbox/issues/4)
[#6](https://github.com/Archont561/pixi-sandbox/issues/6)
[#6](https://github.com/Archont561/pixi-sandbox/issues/6)

### Build System

* **task125:** evaluate rattler with patched ZIP dependencies
([6660542](https://github.com/Archont561/pixi-sandbox/commit/6660542926cefe68dcecdc17927b95cb93c177e9))
* **task125:** provision verified sources for native package migration
([05be8c3](https://github.com/Archont561/pixi-sandbox/commit/05be8c31aecf408ad63cf112c3ff75129774baf3))

### CI

* **release:** name the publish environment after what it gates (#174)
([8e9d499](https://github.com/Archont561/pixi-sandbox/commit/8e9d49965a497c40bdc1bd6ea222004c37d13804)),
closes [#174](https://github.com/Archont561/pixi-sandbox/issues/174)

## [v0.7.0](https://github.com/Archont561/pixi-sandbox/compare/v0.6.2...v0.7.0) (2026-10-15)

### Features

* **pack:** refuse a non-wheel PyPI dependency before creating output
([eef8e70](https://github.com/Archont561/pixi-sandbox/commit/eef8e70710a43f5842f9127b34c35e89ca583842)),
closes [#157](https://github.com/Archont561/pixi-sandbox/issues/157)
* **git-hooks:** add local main pre-push check
([e372421](https://github.com/Archont561/pixi-sandbox/commit/e372421eea7fc27c90010998127fccab958c8f06))
* **wrap-payload:** enforce built package blob budget
([1942e71](https://github.com/Archont561/pixi-sandbox/commit/1942e712dc125399f545f17371e8f8e323da1764))
* **payload:** build and publish wrapped payloads with rattler-build (TASK-78 Slice 3)
([4e07097](https://github.com/Archont561/pixi-sandbox/commit/4e0709773b4e2ac79fe714924dd6085f70945e8d)),
closes [#12](https://github.com/Archont561/pixi-sandbox/issues/12)
* **payload:** add the wrap-payload verb and recipe staging (TASK-78 Slice 2)
([584cb2e](https://github.com/Archont561/pixi-sandbox/commit/584cb2e67fa285ad480eb16f007e27271d093306)),
closes [#13](https://github.com/Archont561/pixi-sandbox/issues/13)
* **payload:** declare, prepare and digest-gate a payload (TASK-78 Slice 1)
([8832ba1](https://github.com/Archont561/pixi-sandbox/commit/8832ba1022f329df1053f927079dcec1d541510f)),
closes [#13](https://github.com/Archont561/pixi-sandbox/issues/13)
* **core:** validate, budget and wrap prebuilt payloads as conda packages
([9878e4f](https://github.com/Archont561/pixi-sandbox/commit/9878e4f87e6c6db0bec31263ef82034237a33121)),
closes [#7](https://github.com/Archont561/pixi-sandbox/issues/7)
[#9](https://github.com/Archont561/pixi-sandbox/issues/9)
* **starter:** title each published snapshot "Initial commit"
([cd54b23](https://github.com/Archont561/pixi-sandbox/commit/cd54b238bc679542ece32bdd81f2c3f94f9fc9c9))
* **starter:** replace main with a leased single-commit snapshot
([1601223](https://github.com/Archont561/pixi-sandbox/commit/160122314d278fdc3bbf4c6dc4f9d2eec162f609))

### Fixes

* **wrap-payload:** refuse a non-UTF-8 payload name as core does
([bd6ae27](https://github.com/Archont561/pixi-sandbox/commit/bd6ae276db2c5e4639a36d87145a1afc3077c784))
* **wrap-payload:** stage a single-file payload as a payload directory
([6169449](https://github.com/Archont561/pixi-sandbox/commit/61694496a8c70242f86c509d66eec4f8fc1dd049)),
closes [#2](https://github.com/Archont561/pixi-sandbox/issues/2)
* **test:** make the payload round-trip oracle match a conda package (#149)
([b2150a2](https://github.com/Archont561/pixi-sandbox/commit/b2150a2e77c39dc7476c2f4a2978a76e13c59189)),
closes [#149](https://github.com/Archont561/pixi-sandbox/issues/149)
[#7](https://github.com/Archont561/pixi-sandbox/issues/7)
[#371](https://github.com/Archont561/pixi-sandbox/issues/371)
[#3](https://github.com/Archont561/pixi-sandbox/issues/3)
* **test:** tighten payload round-trip linting
([0766725](https://github.com/Archont561/pixi-sandbox/commit/0766725c135741fbece3776e5f08bc448f359e22))
* **test:** use the built binary path in round-trip proof
([4546470](https://github.com/Archont561/pixi-sandbox/commit/45464704bee368dafa43d32a53d9ef94f73ae8b4))
* **payload:** keep generated bytes under ignored pixi state
([0144080](https://github.com/Archont561/pixi-sandbox/commit/0144080147ce4bf420b5346da70fa1d6e9605982))
* **starter:** disable Cargo vendoring for non-Rust projects
([95d097a](https://github.com/Archont561/pixi-sandbox/commit/95d097ad9388bcdcf2d0fc541986032c670e82a5))

### Refactoring

* **backlog:** move every .knowledge file into backlog
([740004c](https://github.com/Archont561/pixi-sandbox/commit/740004c6d9f380827be7f3e265f9dcb56c4d20f4))

### Documentation

* **context:** record task-115 closure
([ef6eae1](https://github.com/Archont561/pixi-sandbox/commit/ef6eae1a1a90cc628e3fafb9e1bde526e1c59066))
* **backlog:** close task-115
([ca57ee6](https://github.com/Archont561/pixi-sandbox/commit/ca57ee64ee6f2cdc2223ad45c8438bb65c1891e0))
* **backlog:** defer task-85 dry-run proof
([0af80f2](https://github.com/Archont561/pixi-sandbox/commit/0af80f285913bce3f62eccc7257ae2d348d24b10))
* document setup-pixi empty default env pitfall; record decision
([63e6ef6](https://github.com/Archont561/pixi-sandbox/commit/63e6ef6abd3c314044116a53173782c8edfc1ed4)),
closes [#158](https://github.com/Archont561/pixi-sandbox/issues/158)
* **backlog:** close TASK-78 on the connected tool-fetch and channel proofs
([d81b5f2](https://github.com/Archont561/pixi-sandbox/commit/d81b5f2cb610cdeb15f0e26e4891a06adbd1efad)),
closes [#12](https://github.com/Archont561/pixi-sandbox/issues/12)
[#11](https://github.com/Archont561/pixi-sandbox/issues/11)
* **backlog:** close TASK-88 and TASK-93 on the connected pypi round trip
([b6031a0](https://github.com/Archont561/pixi-sandbox/commit/b6031a0d61a80b3b01ec0ece43cd66007b8714a4)),
closes [#8](https://github.com/Archont561/pixi-sandbox/issues/8)
[#4](https://github.com/Archont561/pixi-sandbox/issues/4)
* **backlog:** close TASK-112 on the green lint gate
([25ca500](https://github.com/Archont561/pixi-sandbox/commit/25ca50093c8bff067202c5a7b5606f18bb996967)),
closes [#5](https://github.com/Archont561/pixi-sandbox/issues/5)
[#2](https://github.com/Archont561/pixi-sandbox/issues/2)
* clarify HTTP adapter boundaries
([54ff983](https://github.com/Archont561/pixi-sandbox/commit/54ff983fc95246500b3680c8064fad209b189804))
* **context:** record the 2026-10-14 session that landed PR #151
([7d67cac](https://github.com/Archont561/pixi-sandbox/commit/7d67cac8c3c003d6d77069c5454a4fdffc4f6816)),
closes [#151](https://github.com/Archont561/pixi-sandbox/issues/151)
[#11](https://github.com/Archont561/pixi-sandbox/issues/11)
[#12](https://github.com/Archont561/pixi-sandbox/issues/12)
* **backlog:** close TASK-111 on the guard's first live verdicts
([ca4f440](https://github.com/Archont561/pixi-sandbox/commit/ca4f440c2d62999a6d7d2dc12e1013d28a7c4ca6)),
closes [#5](https://github.com/Archont561/pixi-sandbox/issues/5)
[#151](https://github.com/Archont561/pixi-sandbox/issues/151)
[#151](https://github.com/Archont561/pixi-sandbox/issues/151)
[#148](https://github.com/Archont561/pixi-sandbox/issues/148)
[#144](https://github.com/Archont561/pixi-sandbox/issues/144)
[#5](https://github.com/Archont561/pixi-sandbox/issues/5)
* **adr:** accept ADR-28 and check TASK-78 AC#1
([ab62ff4](https://github.com/Archont561/pixi-sandbox/commit/ab62ff43028f7ca7a1e3cdc286c17dd858d0f333)),
closes [#1](https://github.com/Archont561/pixi-sandbox/issues/1)
* **context:** record that this session is a connected Codespaces container
([ba54fe1](https://github.com/Archont561/pixi-sandbox/commit/ba54fe133b50d219423eecd1f32e156bc7b2d7a6))
* **context:** record the 2026-10-13 session that landed PR #150
([c3d09be](https://github.com/Archont561/pixi-sandbox/commit/c3d09beb6c675f34d004e2e7b2167781cfd06efc)),
closes [#150](https://github.com/Archont561/pixi-sandbox/issues/150)
[#8](https://github.com/Archont561/pixi-sandbox/issues/8)
* **backlog:** add TASK-111 to enforce how main is integrated
([a763348](https://github.com/Archont561/pixi-sandbox/commit/a7633480cf0da21512870c488b74097a1b9781e1)),
closes [#144](https://github.com/Archont561/pixi-sandbox/issues/144)
[#146](https://github.com/Archont561/pixi-sandbox/issues/146)
* **backlog:** note TASK-88 AC#8's local fix landed as TASK-93
([a5ac9e1](https://github.com/Archont561/pixi-sandbox/commit/a5ac9e10038478e7587e9d005a46a99c27fe0273)),
closes [#8](https://github.com/Archont561/pixi-sandbox/issues/8)
[#8](https://github.com/Archont561/pixi-sandbox/issues/8)
* **backlog:** audit TASK-78 acceptance criteria on main
([39f4767](https://github.com/Archont561/pixi-sandbox/commit/39f4767ea754f7019dfd86ec1d184736fa209ece)),
closes [#2](https://github.com/Archont561/pixi-sandbox/issues/2)
[#4](https://github.com/Archont561/pixi-sandbox/issues/4)
[#6](https://github.com/Archont561/pixi-sandbox/issues/6)
[#7](https://github.com/Archont561/pixi-sandbox/issues/7)
[#8](https://github.com/Archont561/pixi-sandbox/issues/8)
[#9](https://github.com/Archont561/pixi-sandbox/issues/9)
[#10](https://github.com/Archont561/pixi-sandbox/issues/10)
[#13](https://github.com/Archont561/pixi-sandbox/issues/13)
[#1](https://github.com/Archont561/pixi-sandbox/issues/1)
[#11](https://github.com/Archont561/pixi-sandbox/issues/11)
[#12](https://github.com/Archont561/pixi-sandbox/issues/12)
* **context:** record the payload round-trip repair and the empty-directory limitation
([f815949](https://github.com/Archont561/pixi-sandbox/commit/f8159493dfac00b324686f6a0acb9198323f6f3f)),
closes [#371](https://github.com/Archont561/pixi-sandbox/issues/371)
[#3](https://github.com/Archont561/pixi-sandbox/issues/3)
[#3](https://github.com/Archont561/pixi-sandbox/issues/3)
[#5](https://github.com/Archont561/pixi-sandbox/issues/5)
* **backlog:** check TASK-78 AC#3 and AC#5 on the green round trip
([d685a76](https://github.com/Archont561/pixi-sandbox/commit/d685a765d4787633e4eb7a471f68d493d2089e24)),
closes [#3](https://github.com/Archont561/pixi-sandbox/issues/3)
[#5](https://github.com/Archont561/pixi-sandbox/issues/5)
[#3](https://github.com/Archont561/pixi-sandbox/issues/3)
[#5](https://github.com/Archont561/pixi-sandbox/issues/5)
[#1](https://github.com/Archont561/pixi-sandbox/issues/1)
* **context:** record the codecov/patch diagnosis and the TASK-78 coverage follow-up
([3d464fb](https://github.com/Archont561/pixi-sandbox/commit/3d464fbabdbbca749ef29444b91c4e2f5b4f46a1)),
closes [#1](https://github.com/Archont561/pixi-sandbox/issues/1)
[#3](https://github.com/Archont561/pixi-sandbox/issues/3)
[#5](https://github.com/Archont561/pixi-sandbox/issues/5)
[#8](https://github.com/Archont561/pixi-sandbox/issues/8)
[#10](https://github.com/Archont561/pixi-sandbox/issues/10)
[#11](https://github.com/Archont561/pixi-sandbox/issues/11)
* **payload:** add the payload-to-channel guide (TASK-78 AC#8, AC#10)
([13d64ef](https://github.com/Archont561/pixi-sandbox/commit/13d64ef8606278c8078453bf7223cf9cd72b18e2)),
closes [#8](https://github.com/Archont561/pixi-sandbox/issues/8)
[#10](https://github.com/Archont561/pixi-sandbox/issues/10)
* **payload:** record the verified mirrors and recipe findings (TASK-78 Slice 4)
([a9ef72e](https://github.com/Archont561/pixi-sandbox/commit/a9ef72ed3a47ed41487a236eec86d4b4750a1181))
* **migration:** opt the historical v0.3.x pins out of the release-reference scan
([a6da557](https://github.com/Archont561/pixi-sandbox/commit/a6da5574e20b8af390db0720366fb33b749fd4ab))
* **backlog:** close task-110 docs CI checks and favicon
([81e9f2c](https://github.com/Archont561/pixi-sandbox/commit/81e9f2c804616fda804f8fe95d52e7222916b0ea))
* **ci:** run four docs quality gates in the docs workflow
([3fac827](https://github.com/Archont561/pixi-sandbox/commit/3fac827b8eface116cb96a989c0f1e5bb75e3935))
* **a11y:** hide decorative icons, fix contrast failures, ship the favicon
([03ad9cf](https://github.com/Archont561/pixi-sandbox/commit/03ad9cffa7c7dbb84c2c27c43284e95b1b251415)),
closes [#10](https://github.com/Archont561/pixi-sandbox/issues/10)
[#3](https://github.com/Archont561/pixi-sandbox/issues/3)
[#047857](https://github.com/Archont561/pixi-sandbox/issues/047857)
[#1](https://github.com/Archont561/pixi-sandbox/issues/1)
* **backlog:** close task-109 guide template standardization
([e97ea87](https://github.com/Archont561/pixi-sandbox/commit/e97ea8779b2b75fc577156f893846d506d5109cd))
* **guides:** standardize every guide on the outcome-to-next-steps template
([ebc18b9](https://github.com/Archont561/pixi-sandbox/commit/ebc18b91129e049d78cb024831a6ecfb952fe915))
* **backlog:** close task-108 wider reference content mode
([5818e40](https://github.com/Archont561/pixi-sandbox/commit/5818e404f817c4b2216107c33069b8249e202cbe))
* **layout:** give reference pages a wider content column via a ContentPanel override
([230b042](https://github.com/Archont561/pixi-sandbox/commit/230b042f0a595c7b816cc272df6a944002ec7e59))
* **backlog:** close task-107 homepage Icons & Stack replacement
([90a5194](https://github.com/Archont561/pixi-sandbox/commit/90a51944f818a77ae1d0f5fd3ae7ddb755df8eea))
* **site:** replace the homepage Icons & Stack section with security and support
([da1484f](https://github.com/Archont561/pixi-sandbox/commit/da1484ff9149820000aded00e240a8abe7b40f3b))
* **backlog:** close task-106 printable operator checklist
([6b5e468](https://github.com/Archont561/pixi-sandbox/commit/6b5e468236949692af0ead53b8f926cd211d08e6))
* **restore:** add a printable operator checklist to the airlock restore page
([f26b032](https://github.com/Archont561/pixi-sandbox/commit/f26b032604ceb436063bddfaf8230ffc333e3fc1))
* **backlog:** close task-105 migration matrix
([5a0f159](https://github.com/Archont561/pixi-sandbox/commit/5a0f159526880df07685fb983c6d0ecf47555c48))
* **guides:** expand the composite-Actions migration guide into a full matrix
([c0f02b3](https://github.com/Archont561/pixi-sandbox/commit/c0f02b3ee82bd8fb3ec1ca26b373e2feafa9cb0c))
* align generated-workflow descriptions with the pipeline/fetch-release/upgrade entrypoints
([55ac7d0](https://github.com/Archont561/pixi-sandbox/commit/55ac7d0cda5ed5fa63d1b852185add6f9af091b2))
* **content:** publish compatibility and releases with a footer version badge
([3afb918](https://github.com/Archont561/pixi-sandbox/commit/3afb918dd3ae32b29dc1243f4df08a333395cdee))
* **reference:** split the configuration reference into overview and policy
([ddafb24](https://github.com/Archont561/pixi-sandbox/commit/ddafb24ef4106236876394e18dbc5517f07d62ec))
* **reference:** split the CLI reference into five lifecycle pages
([684f1f9](https://github.com/Archont561/pixi-sandbox/commit/684f1f9d8ff08cd4790ad87a9fe4cfa85f59c53c))
* **ux:** add machine and network context chips to command blocks
([f6e7c9a](https://github.com/Archont561/pixi-sandbox/commit/f6e7c9a9f54ee724b6bf8a04372f76d8535a1dd6))
* **content:** publish troubleshooting and incident response
([3f7ef3f](https://github.com/Archont561/pixi-sandbox/commit/3f7ef3f8162f3fadadff209b6dc83b9920e99222))
* **content:** publish the glossary and security boundaries
([e18d558](https://github.com/Archont561/pixi-sandbox/commit/e18d558785f3b5edc89247f4eb859303d84e1f3c))
* **homepage:** add role-based entry cards under the hero
([6c4558e](https://github.com/Archont561/pixi-sandbox/commit/6c4558eaa33c9f71f40a39a3a3bd00989e6b2e0a))
* **content:** ship the first verified offline bundle guide
([cd226e3](https://github.com/Archont561/pixi-sandbox/commit/cd226e37add69b0f30f898190e643d297e8989d9))
* **site:** restructure the docs IA around roles and lifecycle phases
([9cc1994](https://github.com/Archont561/pixi-sandbox/commit/9cc19944e22910cff2189c49b040431761719ac2))
* **backlog:** capture the docs audit and groom its refactor into tasks
([eb83a70](https://github.com/Archont561/pixi-sandbox/commit/eb83a70d89d88ecf14ea2244e9721b3cf8786f52))
* **backlog:** record TASK-78 slices 0-1 and the open verification items
([f8908e5](https://github.com/Archont561/pixi-sandbox/commit/f8908e58d52cd758b5e3bf16598c1570ff2ca581)),
closes [#1](https://github.com/Archont561/pixi-sandbox/issues/1)
* **adr:** propose ADR-28 for TASK-78 and fix stale decision references
([7e6c63b](https://github.com/Archont561/pixi-sandbox/commit/7e6c63b9393fb21dbf0d69f258b520e0b917fe0e)),
closes [#1](https://github.com/Archont561/pixi-sandbox/issues/1)
* **backlog:** settle TASK-78 on a wrap-payload verb and a mirrored channel
([3411dae](https://github.com/Archont561/pixi-sandbox/commit/3411dae935ee57c58de7242490c52c9c89aff352)),
closes [#11](https://github.com/Archont561/pixi-sandbox/issues/11)
[#12](https://github.com/Archont561/pixi-sandbox/issues/12)
[#13](https://github.com/Archont561/pixi-sandbox/issues/13)
[#11](https://github.com/Archont561/pixi-sandbox/issues/11)
* **xtask:** describe starter-publish as a leased snapshot
([5871f11](https://github.com/Archont561/pixi-sandbox/commit/5871f112048993c623a95d1c52f7f3fa30fbcdc6))
* **context:** record single-commit starter policy and rollout boundary
([4a6ff8d](https://github.com/Archont561/pixi-sandbox/commit/4a6ff8d8087d229d6acaffd620407a171f54ab9f))
* **context:** record starter pack fix and pending remote proof
([cd4b56b](https://github.com/Archont561/pixi-sandbox/commit/cd4b56b5236c4e0f99822537365e39648e54e310))
* **readme:** lead with starter quickstart and concise capability guide
([527019d](https://github.com/Archont561/pixi-sandbox/commit/527019dd40f7c2f37198018d3bebdac949410e75))
* **context:** record v0.6.2 evidence review and handoff
([d02bae6](https://github.com/Archont561/pixi-sandbox/commit/d02bae60412bf636c1e1625e84586a5c74932efe))
* **backlog:** distinguish task-85 publish proof from dry run
([e050c81](https://github.com/Archont561/pixi-sandbox/commit/e050c811357adec891a4e93001d906e91a0c5a9f))
* **backlog:** record task-74 starter publication evidence
([9390469](https://github.com/Archont561/pixi-sandbox/commit/9390469983f35b2ac6f177bcddc27ffe979d2a68))
* **backlog:** close task-89 with v0.6.2 release proof
([5737d69](https://github.com/Archont561/pixi-sandbox/commit/5737d69e2ccd3f563f25fa6b6b8a9bdb45bc3499))

### CI

* **deps:** bump the actions-patch group in the workflows and generated templates
([93b5c3c](https://github.com/Archont561/pixi-sandbox/commit/93b5c3cb4cafbdf36d7a06eb405c6cb63d8c16a6)),
closes [#147](https://github.com/Archont561/pixi-sandbox/issues/147)
[#147](https://github.com/Archont561/pixi-sandbox/issues/147)
* **main:** guard how main is integrated (TASK-111)
([fec5fd7](https://github.com/Archont561/pixi-sandbox/commit/fec5fd725402a240c7bf1999eef00e490b31e76a)),
closes [#144](https://github.com/Archont561/pixi-sandbox/issues/144)
[#146](https://github.com/Archont561/pixi-sandbox/issues/146)

### [v0.6.2](https://github.com/Archont561/pixi-sandbox/compare/v0.6.1...v0.6.2) (2026-10-11)

#### Fixes

* **verify:** collapse regenerated uv direct-url metadata
([8a17a26](https://github.com/Archont561/pixi-sandbox/commit/8a17a26fca1c5f1a514fbe296d00ce194571e31b))
* **release-lane:** publish a complete starter and gate the release on the consumer proof (#133)
([46efd1a](https://github.com/Archont561/pixi-sandbox/commit/46efd1a73aee7af5231aa056f2029bf12b6a749b)),
closes [#133](https://github.com/Archont561/pixi-sandbox/issues/133)

#### Refactoring

* **msrv:** declare rust 1.88 and clear the collapsible_if debt (#139)
([3d01bad](https://github.com/Archont561/pixi-sandbox/commit/3d01bad511da9df1f6ccde4f3cb1768af2d8927e)),
closes [#139](https://github.com/Archont561/pixi-sandbox/issues/139)
* **pixi-sandbox:** consolidate GitHub adapters
([297a257](https://github.com/Archont561/pixi-sandbox/commit/297a2576ed1b485d7c38822b836d85347b11d673))
* **release:** port the consumer proof into xtask subcommands (#136)
([3665185](https://github.com/Archont561/pixi-sandbox/commit/36651855f86b3d70723a98270af462663526df93)),
closes [#136](https://github.com/Archont561/pixi-sandbox/issues/136)
[#3](https://github.com/Archont561/pixi-sandbox/issues/3)
[#3](https://github.com/Archont561/pixi-sandbox/issues/3)
[#1](https://github.com/Archont561/pixi-sandbox/issues/1)
[#2](https://github.com/Archont561/pixi-sandbox/issues/2)
[#4](https://github.com/Archont561/pixi-sandbox/issues/4)
[#3](https://github.com/Archont561/pixi-sandbox/issues/3)
[#2](https://github.com/Archont561/pixi-sandbox/issues/2)

#### Documentation

* **backlog:** TASK-85 AC#6 re-attempted, dispatch still 403 (token capability)
([a4c0022](https://github.com/Archont561/pixi-sandbox/commit/a4c0022c0d30175c905ed2e509e9da99e0934144)),
closes [#6](https://github.com/Archont561/pixi-sandbox/issues/6)
* **backlog:** TASK-89 record connected-host lifecycle evidence, AC#3 still open
([bf794ce](https://github.com/Archont561/pixi-sandbox/commit/bf794ced8f8be20c8818c91101451596c7c33d3a)),
closes [#3](https://github.com/Archont561/pixi-sandbox/issues/3)
* **backlog:** TASK-90 spike verdict — adopt self_update transport, keep our replace
([0d1e691](https://github.com/Archont561/pixi-sandbox/commit/0d1e69131f577cbdc5ee971f667b007e9a268743))
* **backlog:** TASK-76 AC#8 proven on released v0.6.1 consumer-shaped run
([9194f38](https://github.com/Archont561/pixi-sandbox/commit/9194f38261b86ebf29f082517ca58857e9b962cb)),
closes [#8](https://github.com/Archont561/pixi-sandbox/issues/8)
* **backlog:** record TASK-88 AC#8 real-round-trip failure, file TASK-93
([e2db1f6](https://github.com/Archont561/pixi-sandbox/commit/e2db1f6958cb007358439be6fe17967409d7cd0b)),
closes [#8](https://github.com/Archont561/pixi-sandbox/issues/8)
* **context:** record the TASK-89 session and the check-13 vacuity it found (#137)
([2883243](https://github.com/Archont561/pixi-sandbox/commit/2883243e8c4b53677c9ce3bdcc2ad3b9c6bdaea8)),
closes [#137](https://github.com/Archont561/pixi-sandbox/issues/137)
[#136](https://github.com/Archont561/pixi-sandbox/issues/136)
[#3](https://github.com/Archont561/pixi-sandbox/issues/3)
* record the release-lane session and the two tasks it leaves open (#134)
([7cb3d54](https://github.com/Archont561/pixi-sandbox/commit/7cb3d5438b8aa2d501f01650d06c59c6726d59da)),
closes [#134](https://github.com/Archont561/pixi-sandbox/issues/134)
[#1](https://github.com/Archont561/pixi-sandbox/issues/1)
[#2](https://github.com/Archont561/pixi-sandbox/issues/2)
[#6](https://github.com/Archont561/pixi-sandbox/issues/6)
[#9](https://github.com/Archont561/pixi-sandbox/issues/9)
[#10](https://github.com/Archont561/pixi-sandbox/issues/10)
[#7](https://github.com/Archont561/pixi-sandbox/issues/7)
[#8](https://github.com/Archont561/pixi-sandbox/issues/8)
[#8](https://github.com/Archont561/pixi-sandbox/issues/8)
[#8](https://github.com/Archont561/pixi-sandbox/issues/8)

#### Build System

* **deps:** declare self_update for the self-update evaluation (#135)
([00ef66e](https://github.com/Archont561/pixi-sandbox/commit/00ef66e4fa830c69baa8c30f1e15c86f8eaef816)),
closes [#135](https://github.com/Archont561/pixi-sandbox/issues/135)
[#135](https://github.com/Archont561/pixi-sandbox/issues/135)

### [v0.6.1](https://github.com/Archont561/pixi-sandbox/compare/v0.6.0...v0.6.1) (2026-10-10)

#### Features

* make pixi-sandbox the single entrypoint in generated workflows
([b9d5d0a](https://github.com/Archont561/pixi-sandbox/commit/b9d5d0a22c94b4de962ed9af97975420ba0bfacd))
* **host-requirements:** declare, carry, diagnose and document host requirements (#120)
([469f408](https://github.com/Archont561/pixi-sandbox/commit/469f408542895e5012729c8e8122bdede6948692)),
closes [#120](https://github.com/Archont561/pixi-sandbox/issues/120)
[#117](https://github.com/Archont561/pixi-sandbox/issues/117)
[#119](https://github.com/Archont561/pixi-sandbox/issues/119)
* **starter:** publish a release-pinned starter template on every release
([b359b5f](https://github.com/Archont561/pixi-sandbox/commit/b359b5ff74298b4ab1da23948ba0a099de93e66d))
* **xtask:** add the starter scaffold and its release-pinning checks
([02c0684](https://github.com/Archont561/pixi-sandbox/commit/02c06842f4de988e1296b4674e4204ab83912421))

#### Fixes

* **pixi-sandbox-core:** stop digesting uv-regenerated install metadata in the oracle
([73fbbd4](https://github.com/Archont561/pixi-sandbox/commit/73fbbd422ef37e30eacd96a06c9444efd342abaa)),
closes [#128](https://github.com/Archont561/pixi-sandbox/issues/128)
* **pixi-sandbox:** resolve the pack cast sites
([7ae7420](https://github.com/Archont561/pixi-sandbox/commit/7ae74208f6af0e51d4213896669caf5726166c7e)),
closes [#8](https://github.com/Archont561/pixi-sandbox/issues/8)
* **pixi-sandbox:** resolve the mechanical pedantic findings
([eb854ea](https://github.com/Archont561/pixi-sandbox/commit/eb854ea3a283d2d8d6222943ee4a7a8738f9b0fa))
* **pixi-sandbox-git:** clear the pedantic doc and builder debt
([74bf640](https://github.com/Archont561/pixi-sandbox/commit/74bf640d9b0c2fff267450e07b96f6a991e2c98d)),
closes [#4](https://github.com/Archont561/pixi-sandbox/issues/4)
* **pixi-sandbox-core:** resolve the transport-size cast sites
([b3a195e](https://github.com/Archont561/pixi-sandbox/commit/b3a195ed48f982900efe6053fb22a84b83b252b0)),
closes [#8](https://github.com/Archont561/pixi-sandbox/issues/8)
* **xtask:** apply machine-applicable pedantic fixes
([38d6dc3](https://github.com/Archont561/pixi-sandbox/commit/38d6dc3bcfc6ff866e4411421f219c30f002104e))
* **pixi-sandbox:** apply machine-applicable pedantic fixes
([afa06c6](https://github.com/Archont561/pixi-sandbox/commit/afa06c63d1c6d9bafcaf947c63fe4edd667c1121))
* **pixi-sandbox-core:** apply machine-applicable pedantic fixes
([08a7e76](https://github.com/Archont561/pixi-sandbox/commit/08a7e76552af2d2fc41e843a946112ee66726293))
* **starter:** verify the tree git would commit, not the working directory (#125)
([ce47e41](https://github.com/Archont561/pixi-sandbox/commit/ce47e414c6650358f3dd14606c2fea8d53191427)),
closes [#125](https://github.com/Archont561/pixi-sandbox/issues/125)
[#6](https://github.com/Archont561/pixi-sandbox/issues/6)
* **core:** reject manifest paths with no file name instead of panicking in shard.rs (TASK-80)
(#122)
([5a4b4a7](https://github.com/Archont561/pixi-sandbox/commit/5a4b4a79a0eea6dfd9db3adbc4ccb254ab360a1d)),
closes [#122](https://github.com/Archont561/pixi-sandbox/issues/122)
[#5](https://github.com/Archont561/pixi-sandbox/issues/5)
[#1](https://github.com/Archont561/pixi-sandbox/issues/1)
[#3](https://github.com/Archont561/pixi-sandbox/issues/3)
[#3](https://github.com/Archont561/pixi-sandbox/issues/3)
[#4](https://github.com/Archont561/pixi-sandbox/issues/4)
[#1](https://github.com/Archont561/pixi-sandbox/issues/1)
[#2](https://github.com/Archont561/pixi-sandbox/issues/2)
[#3](https://github.com/Archont561/pixi-sandbox/issues/3)
[#4](https://github.com/Archont561/pixi-sandbox/issues/4)
[#5](https://github.com/Archont561/pixi-sandbox/issues/5)
[#6](https://github.com/Archont561/pixi-sandbox/issues/6)
* **ci:** verify starter main ref before checkout (#119) (#119)
([a42bcac](https://github.com/Archont561/pixi-sandbox/commit/a42bcace23fa06b91a4765080527a5e8f0d05a5e)),
closes [#119](https://github.com/Archont561/pixi-sandbox/issues/119)
[#119](https://github.com/Archont561/pixi-sandbox/issues/119)
* **ci:** forward starter workflow task arguments (#118)
([57a78df](https://github.com/Archont561/pixi-sandbox/commit/57a78df6e2de1fe4ef364b75831b7b67045d7300)),
closes [#118](https://github.com/Archont561/pixi-sandbox/issues/118)

#### Refactoring

* **pixi-sandbox:** promote the tools update core into the lib
([7d8e3cb](https://github.com/Archont561/pixi-sandbox/commit/7d8e3cb84c975ac75bd2390709bb7ab939577dcd)),
closes [#9](https://github.com/Archont561/pixi-sandbox/issues/9)
* **xtask:** extract the relock render lint from lint_generated_workflow
([0aa99b2](https://github.com/Archont561/pixi-sandbox/commit/0aa99b2517d2dd648875c6920cde37cb0fd1849c))
* **xtask:** apply the mechanical pedantic fixes
([c915b0d](https://github.com/Archont561/pixi-sandbox/commit/c915b0d48b39c58081da9ad4926eaf2fc85229f2))
* **branch-docs:** extract render's pieces and guide renderers
([f0f4868](https://github.com/Archont561/pixi-sandbox/commit/f0f486886873176576be99eea58d182aef7dae5c))
* **pack:** extract assemble_artifacts' three phases
([bf71955](https://github.com/Archont561/pixi-sandbox/commit/bf71955f7da5b73e3c22d858386dd9e40f6b9c21))
* **doctor:** extract as_json's report sections
([d7ef844](https://github.com/Archont561/pixi-sandbox/commit/d7ef844fab53b3d8c2c29b5b42721c8931c5a3f3))
* **tools-update:** extract refresh's per-platform pin update
([fc321ec](https://github.com/Archont561/pixi-sandbox/commit/fc321ec36e6467c59f1e3731ea65d50b12cbfcbf)),
closes [#5](https://github.com/Archont561/pixi-sandbox/issues/5)
* **restore:** extract run's unpacker, vendor and cargo-wiring phases
([3981801](https://github.com/Archont561/pixi-sandbox/commit/39818011445ef9377e292200fabbf3036192445b)),
closes [#5](https://github.com/Archont561/pixi-sandbox/issues/5)
* **doctor:** extract run's verification, report and failure phases
([acd3f79](https://github.com/Archont561/pixi-sandbox/commit/acd3f79227feaf0f504d58889c11e48e9506da4e)),
closes [#5](https://github.com/Archont561/pixi-sandbox/issues/5)
* **restore:** extract register_user_tools' registration report
([93dc616](https://github.com/Archont561/pixi-sandbox/commit/93dc616c7dcadac232d14d3af26169bca5f01e19)),
closes [#5](https://github.com/Archont561/pixi-sandbox/issues/5)
* **doctor:** extract print_human's six report sections
([69ee561](https://github.com/Archont561/pixi-sandbox/commit/69ee56134b6d60eec90c5d5474b30edb1a155a75)),
closes [#5](https://github.com/Archont561/pixi-sandbox/issues/5)
* **pixi-sandbox-core:** extract evaluate's libc and capability phases
([4b5dfaf](https://github.com/Archont561/pixi-sandbox/commit/4b5dfaf208e6047f0843353f92fcf139a272c0bb))
* **pixi-sandbox-core:** extract verify_env_restored's four phases
([7cdace6](https://github.com/Archont561/pixi-sandbox/commit/7cdace61245c09eb6977f2097b5d307e95d5805b))
* **pixi-sandbox-core:** resolve the audited pedantic judgement items
([577eb52](https://github.com/Archont561/pixi-sandbox/commit/577eb52c136249b109375c71faf33787ec147cdd))
* **pack:** promote library policy and migrate BDD bindings (#127) (#127)
([0a036ba](https://github.com/Archont561/pixi-sandbox/commit/0a036ba2649e8fae684930a3fc423fe0a4a81586)),
closes [#127](https://github.com/Archont561/pixi-sandbox/issues/127)
[#127](https://github.com/Archont561/pixi-sandbox/issues/127)
[#126](https://github.com/Archont561/pixi-sandbox/issues/126)

#### Documentation

* **context:** record the TASK-88 session and the next session prompt
([7f98e22](https://github.com/Archont561/pixi-sandbox/commit/7f98e22d0a255c9e10ab84048e856add9a139d68)),
closes [#128](https://github.com/Archont561/pixi-sandbox/issues/128)
[#109](https://github.com/Archont561/pixi-sandbox/issues/109)
* **backlog:** file TASK-88 for issue #128 at 7 of 8
([7bd0582](https://github.com/Archont561/pixi-sandbox/commit/7bd0582af4d4759451febf486bf1a61c8f5d411c)),
closes [#128](https://github.com/Archont561/pixi-sandbox/issues/128)
[#1](https://github.com/Archont561/pixi-sandbox/issues/1)
[#7](https://github.com/Archont561/pixi-sandbox/issues/7)
[#8](https://github.com/Archont561/pixi-sandbox/issues/8)
* **knowledge:** record D21 on uv-regenerated install metadata in the oracle
([ef45808](https://github.com/Archont561/pixi-sandbox/commit/ef45808966d89cc28074fb68c65c4d35239cf9e5)),
closes [#128](https://github.com/Archont561/pixi-sandbox/issues/128)
* **context:** record the close-out session and the next session prompt
([0450668](https://github.com/Archont561/pixi-sandbox/commit/0450668d0c386fe459c8dfd6b98fa0b02442bcd6))
* **backlog:** record the starter-lane rehearsal evidence for TASK-85 AC#6 and TASK-76 AC#8
([1f6af1a](https://github.com/Archont561/pixi-sandbox/commit/1f6af1a6cafbd6147657d62cbd658ccc4b7c89ac)),
closes [#6](https://github.com/Archont561/pixi-sandbox/issues/6)
[#8](https://github.com/Archont561/pixi-sandbox/issues/8)
[#6](https://github.com/Archont561/pixi-sandbox/issues/6)
[#8](https://github.com/Archont561/pixi-sandbox/issues/8)
[#131](https://github.com/Archont561/pixi-sandbox/issues/131)
* **context:** add the next session prompt to the TASK-77 closure entry
([4faec30](https://github.com/Archont561/pixi-sandbox/commit/4faec30e589909bc24ab4964dea30ff55f225bfc)),
closes [#8](https://github.com/Archont561/pixi-sandbox/issues/8)
[#6](https://github.com/Archont561/pixi-sandbox/issues/6)
* **backlog:** close TASK-77 with the final evidence entry
([bb8cabe](https://github.com/Archont561/pixi-sandbox/commit/bb8cabe6d7325323bce878c5ed586c57b3b8bb0c)),
closes [#4](https://github.com/Archont561/pixi-sandbox/issues/4)
[#10](https://github.com/Archont561/pixi-sandbox/issues/10)
[#4](https://github.com/Archont561/pixi-sandbox/issues/4)
[#10](https://github.com/Archont561/pixi-sandbox/issues/10)
* **xtask:** shorten first doc paragraphs to a summary line
([db71dbc](https://github.com/Archont561/pixi-sandbox/commit/db71dbc1b7be1d949809f0c473ca0ac71c1fc166)),
closes [#4](https://github.com/Archont561/pixi-sandbox/issues/4)
* **pixi-sandbox:** shorten first doc paragraphs to a summary line
([7b180fa](https://github.com/Archont561/pixi-sandbox/commit/7b180faab62be69b2af8d5f17c4448204c09e699)),
closes [#4](https://github.com/Archont561/pixi-sandbox/issues/4)
* **pixi-sandbox-git:** shorten the first doc paragraph on FileCommit
([cc7748c](https://github.com/Archont561/pixi-sandbox/commit/cc7748c8ee4e6915c08f3b0c39ca5fdd5a54b987)),
closes [#4](https://github.com/Archont561/pixi-sandbox/issues/4)
* **pixi-sandbox-core:** shorten first doc paragraphs to a summary line
([a54b717](https://github.com/Archont561/pixi-sandbox/commit/a54b7174b9e30f56ea3ea0cc13a88159ee8d4d3f)),
closes [#4](https://github.com/Archont561/pixi-sandbox/issues/4)
* **task-77:** record the closure evidence; AC#4 and AC#10 stay open
([11673a2](https://github.com/Archont561/pixi-sandbox/commit/11673a2acb2b067548a2574edb30cca445ec4594)),
closes [#4](https://github.com/Archont561/pixi-sandbox/issues/4)
[#10](https://github.com/Archont561/pixi-sandbox/issues/10)
[#3](https://github.com/Archont561/pixi-sandbox/issues/3)
[#6](https://github.com/Archont561/pixi-sandbox/issues/6)
[#9](https://github.com/Archont561/pixi-sandbox/issues/9)
[#4](https://github.com/Archont561/pixi-sandbox/issues/4)
[#10](https://github.com/Archont561/pixi-sandbox/issues/10)
* **agents:** name the promoted tools_update module in the repo map
([833e5d9](https://github.com/Archont561/pixi-sandbox/commit/833e5d99affad3c4e36094876b99b7b7950f2ff2))
* **xtask:** document the error and panic contracts for the pedantic gate
([68aa870](https://github.com/Archont561/pixi-sandbox/commit/68aa87086e390d30c44d9a0d8e27bef0e903ec0a))
* **context:** TASK-77 checkpoint — core/git/pixi-sandbox clean, xtask remainder handed off
([bf08574](https://github.com/Archont561/pixi-sandbox/commit/bf085748ad1a1f219756c2d83aa150892b3fbb12)),
closes [#1](https://github.com/Archont561/pixi-sandbox/issues/1)
[#2](https://github.com/Archont561/pixi-sandbox/issues/2)
[#5](https://github.com/Archont561/pixi-sandbox/issues/5)
[#7](https://github.com/Archont561/pixi-sandbox/issues/7)
[#8](https://github.com/Archont561/pixi-sandbox/issues/8)
[#9](https://github.com/Archont561/pixi-sandbox/issues/9)
* **pixi-sandbox:** write the missing # Errors and # Panics sections
([a8b9deb](https://github.com/Archont561/pixi-sandbox/commit/a8b9deb8f9fbba1faab92f4e838f496ac4ae9391)),
closes [#4](https://github.com/Archont561/pixi-sandbox/issues/4)
* **pixi-sandbox-core:** write the missing # Errors sections
([2068785](https://github.com/Archont561/pixi-sandbox/commit/206878506b3a5a7856f79ebc98ed4b5121281884)),
closes [#4](https://github.com/Archont561/pixi-sandbox/issues/4)
* **context:** record TASK-76 evidence and the carried AC#8
([b69c002](https://github.com/Archont561/pixi-sandbox/commit/b69c0028fa04a63dd087ca9d554017ce3600cd7c)),
closes [#8](https://github.com/Archont561/pixi-sandbox/issues/8)
[#1](https://github.com/Archont561/pixi-sandbox/issues/1)
[#7](https://github.com/Archont561/pixi-sandbox/issues/7)
[#9](https://github.com/Archont561/pixi-sandbox/issues/9)
[#8](https://github.com/Archont561/pixi-sandbox/issues/8)
* **context:** close the task-85 session with its open findings and a baseline correction
([924b322](https://github.com/Archont561/pixi-sandbox/commit/924b3225594295369846ae19eee08c1b2e5486e3)),
closes [#125](https://github.com/Archont561/pixi-sandbox/issues/125)
[#6](https://github.com/Archont561/pixi-sandbox/issues/6)
* **agents:** correct the repo map and promoted-module list to match the tree (TASK-84) (#123)
([1a5b84d](https://github.com/Archont561/pixi-sandbox/commit/1a5b84daae8f97cea84630eff9c1ebdd31523682)),
closes [#123](https://github.com/Archont561/pixi-sandbox/issues/123)
[#1](https://github.com/Archont561/pixi-sandbox/issues/1)
[#2](https://github.com/Archont561/pixi-sandbox/issues/2)
[#1](https://github.com/Archont561/pixi-sandbox/issues/1)
[#2](https://github.com/Archont561/pixi-sandbox/issues/2)
[#122](https://github.com/Archont561/pixi-sandbox/issues/122)
[#3](https://github.com/Archont561/pixi-sandbox/issues/3)
* **backlog:** audit findings as tasks 79-85, add audit and clean-code-principles skills (#121)
([13d3a86](https://github.com/Archont561/pixi-sandbox/commit/13d3a866a7b8860790351ba44bc13118d44765d6)),
closes [#121](https://github.com/Archont561/pixi-sandbox/issues/121)
* **backlog:** add task-78 and close task-71 on the proven upgrade lane
([542c57b](https://github.com/Archont561/pixi-sandbox/commit/542c57bd0573b4a45f54dc08f04930ffc67ddb32)),
closes [#115](https://github.com/Archont561/pixi-sandbox/issues/115)
* **backlog:** record the starter lane against task-74
([d9ad695](https://github.com/Archont561/pixi-sandbox/commit/d9ad695fb7733fccc42399a50547bf0c5acd5728)),
closes [#3](https://github.com/Archont561/pixi-sandbox/issues/3)
[#4](https://github.com/Archont561/pixi-sandbox/issues/4)
[#7](https://github.com/Archont561/pixi-sandbox/issues/7)
* **backlog:** audit the Rust source for pedantic lint debt and long functions
([b43ba1a](https://github.com/Archont561/pixi-sandbox/commit/b43ba1ae986c925f0af2b5992cf759f48bfb3ed7))
* **backlog:** add task-76 to make pixi-sandbox the entrypoint in generated workflows
([db2d446](https://github.com/Archont561/pixi-sandbox/commit/db2d446525a8e6f618bd57643629c5bd0ee76755)),
closes [#9](https://github.com/Archont561/pixi-sandbox/issues/9)
* **backlog:** record the Castellan 0.6.0 bootstrap and close nothing on it
([b7dfd95](https://github.com/Archont561/pixi-sandbox/commit/b7dfd95a2bcde23f7ac7be807ef085f4a115d836)),
closes [#5](https://github.com/Archont561/pixi-sandbox/issues/5)
[#1](https://github.com/Archont561/pixi-sandbox/issues/1)
[#2](https://github.com/Archont561/pixi-sandbox/issues/2)
[#6](https://github.com/Archont561/pixi-sandbox/issues/6)
* **context:** record the v0.6.0 push-trigger watch and the PR-114 verdicts
([1ba752a](https://github.com/Archont561/pixi-sandbox/commit/1ba752a0b36932beb8a939fa9ea14556986b470e))

#### CI

* bootstrap starter repository before checkout
([f28db3a](https://github.com/Archont561/pixi-sandbox/commit/f28db3ab7400479d2e19e53f97c7a624ef2e3092))

## [v0.6.0](https://github.com/Archont561/pixi-sandbox/compare/v0.5.3...v0.6.0) (2026-10-06)

### Features

* **relock:** publish repaired-head validation verdict
([b9b03ee](https://github.com/Archont561/pixi-sandbox/commit/b9b03ee3798ac9373adfaa86ea0b6ec44b463b8f))

### Fixes

* **ci:** stop auto-release dispatching workflows its own pushes trigger
([766e7f5](https://github.com/Archont561/pixi-sandbox/commit/766e7f53b5b697d2f2e46952152b02122ea8a0cb))
* **ci:** drop the invalid workflows permission scope and guard it
([11ec40b](https://github.com/Archont561/pixi-sandbox/commit/11ec40b3544d9fc198d5ad09980d0ad517c94214))
* **devcontainer:** install opencode from the default environment
([ebe7ed4](https://github.com/Archont561/pixi-sandbox/commit/ebe7ed446ce225abbc2f6443db54fb67c9de529b))
* **workflow:** deliver generated upgrades with a safe fallback (#102)
([8b7e23c](https://github.com/Archont561/pixi-sandbox/commit/8b7e23cc0eb5650f032f025f66e443adb045a871)),
closes [#102](https://github.com/Archont561/pixi-sandbox/issues/102)
* **release:** refresh relock after version bump (#100)
([bcb4df8](https://github.com/Archont561/pixi-sandbox/commit/bcb4df8459a99d4c0ad7eda855be032182dcacab)),
closes [#100](https://github.com/Archont561/pixi-sandbox/issues/100)

### Documentation

* **backlog:** close task 68 on a connected failing publish run
([6b5a514](https://github.com/Archont561/pixi-sandbox/commit/6b5a5145790ec2648dd2d1aec8b3db23868f0ad4)),
closes [#7](https://github.com/Archont561/pixi-sandbox/issues/7)
[#4](https://github.com/Archont561/pixi-sandbox/issues/4)
* **backlog:** close task 47 on the connected upgrade cycle
([145bbfb](https://github.com/Archont561/pixi-sandbox/commit/145bbfb8b1e17b261707daf2df67ddf579512aa0)),
closes [#7](https://github.com/Archont561/pixi-sandbox/issues/7)
[#13](https://github.com/Archont561/pixi-sandbox/issues/13)
[#5](https://github.com/Archont561/pixi-sandbox/issues/5)
[#107](https://github.com/Archont561/pixi-sandbox/issues/107)

### Build System

* **pixi:** add shellcheck so actionlint lints generated Bash locally
([5dfa84e](https://github.com/Archont561/pixi-sandbox/commit/5dfa84eed6f1f2529c28c89d5bb59601510b09b5)),
closes [#103](https://github.com/Archont561/pixi-sandbox/issues/103)

### CI

* serialise the sandbox publisher through a [workflow] policy
([010578b](https://github.com/Archont561/pixi-sandbox/commit/010578b4d707592ae24615e100025d91944aa834)),
closes [#113](https://github.com/Archont561/pixi-sandbox/issues/113)
* serialise release.yml with a tag-scoped concurrency group
([c83f463](https://github.com/Archont561/pixi-sandbox/commit/c83f463d7019330fdaa6ae1fdaaad6044f2b1385)),
closes [#113](https://github.com/Archont561/pixi-sandbox/issues/113)
* grant release workflow permission
([c810785](https://github.com/Archont561/pixi-sandbox/commit/c810785726fb09ef6c0f92af660b65fb603d7820))
* **deps:** bump actions/github-script from 8.0.0 to 9.0.0 (#107)
([7f98261](https://github.com/Archont561/pixi-sandbox/commit/7f98261221843660c7e6a2eb58b5edaebd7c4010)),
closes [#107](https://github.com/Archont561/pixi-sandbox/issues/107)
* **deps:** bump actions/cache from 4.3.0 to 6.1.0 (#106)
([ee21ec4](https://github.com/Archont561/pixi-sandbox/commit/ee21ec43253ecee40c9d7171fff4d0bd7e5ad6c4)),
closes [#106](https://github.com/Archont561/pixi-sandbox/issues/106)

### [v0.5.3](https://github.com/Archont561/pixi-sandbox/compare/v0.5.2...v0.5.3) (2026-10-04)

#### Features

* **workflow:** surface publish failures and preserve healthy transport (#98)
([c0f6a37](https://github.com/Archont561/pixi-sandbox/commit/c0f6a3770f7b50c3c5f6fb97e541d058e8d6c0a0)),
closes [#98](https://github.com/Archont561/pixi-sandbox/issues/98)

#### Fixes

* **pack:** reject only Windows drive prefixes so POSIX colons can pack (#96)
([888af6d](https://github.com/Archont561/pixi-sandbox/commit/888af6dad498ca6a0d5659fd71c681aee5b9c843)),
closes [#96](https://github.com/Archont561/pixi-sandbox/issues/96)
[#95](https://github.com/Archont561/pixi-sandbox/issues/95)
* **pipeline:** guard lock drift and persist diagnostics (#94)
([8e070d4](https://github.com/Archont561/pixi-sandbox/commit/8e070d48c79adb522c8bfc6d162fdb23d6023de0)),
closes [#94](https://github.com/Archont561/pixi-sandbox/issues/94)
* **consumer-proof:** fixture needs a real dependency for pixi-pack to find its platform (#91)
([b13a7fd](https://github.com/Archont561/pixi-sandbox/commit/b13a7fddb9f1dbb9b225c46fcda6b078ad74ea3e)),
closes [#91](https://github.com/Archont561/pixi-sandbox/issues/91)
[#6](https://github.com/Archont561/pixi-sandbox/issues/6)
[#7](https://github.com/Archont561/pixi-sandbox/issues/7)
[#6](https://github.com/Archont561/pixi-sandbox/issues/6)
* **workflow:** complete consumer release proof (#89)
([0160ef4](https://github.com/Archont561/pixi-sandbox/commit/0160ef4a6ef36e247d2c19bdc9086bc4ff198c74)),
closes [#89](https://github.com/Archont561/pixi-sandbox/issues/89)

#### Documentation

* **context:** record the task-69 session and next opening prompt (#97)
([297e4fe](https://github.com/Archont561/pixi-sandbox/commit/297e4fecbf96685316c997f474b3ef1df541e9c3)),
closes [#97](https://github.com/Archont561/pixi-sandbox/issues/97)
[#96](https://github.com/Archont561/pixi-sandbox/issues/96)
* **backlog:** record PR 89 connected proofs (#90)
([e517305](https://github.com/Archont561/pixi-sandbox/commit/e5173053fa120380b3646dbbe1217933f065326c)),
closes [#90](https://github.com/Archont561/pixi-sandbox/issues/90)

#### Build System

* **tooling:** adopt Turbo nextest orchestration (#70) (#99)
([fe130fa](https://github.com/Archont561/pixi-sandbox/commit/fe130fa482a80756f8815e4374ea7e2f8afce684)),
closes [#70](https://github.com/Archont561/pixi-sandbox/issues/70)
[#99](https://github.com/Archont561/pixi-sandbox/issues/99)

### [v0.5.2](https://github.com/Archont561/pixi-sandbox/compare/v0.5.1...v0.5.2) (2026-10-03)

#### Fixes

* **workflow:** add consumer release proof
([bc31d86](https://github.com/Archont561/pixi-sandbox/commit/bc31d866498f05c000909f3eab6bdf4686b0908d))

### [v0.5.1](https://github.com/Archont561/pixi-sandbox/compare/v0.5.0...v0.5.1) (2026-10-03)

#### Features

* **ci:** prove the Windows running-image replace and dispatch publish on relock
([b5918db](https://github.com/Archont561/pixi-sandbox/commit/b5918db93159eb248576c5e8259c9134920b82da)),
closes [#2](https://github.com/Archont561/pixi-sandbox/issues/2)

#### Fixes

* **pack:** refuse self-bins that cannot run standalone (issue 81) (#82)
([3e99f97](https://github.com/Archont561/pixi-sandbox/commit/3e99f97ba1a7f1c8f5ea8fde40519568568c5c4c)),
closes [#82](https://github.com/Archont561/pixi-sandbox/issues/82)

#### Documentation

* **backlog:** task-54 AC#1 — the v0.5.0 asset is standalone, not a trampoline
([6a31874](https://github.com/Archont561/pixi-sandbox/commit/6a31874043b35dc4fe854f4243b9b1c5e0bb92bd)),
closes [#1](https://github.com/Archont561/pixi-sandbox/issues/1)
[#81](https://github.com/Archont561/pixi-sandbox/issues/81)
[#80](https://github.com/Archont561/pixi-sandbox/issues/80)
* **context:** task-54 leftover proofs are connected-host work (#83)
([08ea7ea](https://github.com/Archont561/pixi-sandbox/commit/08ea7ea83312f343ab67e476f0427d869a8f34c8)),
closes [#83](https://github.com/Archont561/pixi-sandbox/issues/83)

## [v0.5.0](https://github.com/Archont561/pixi-sandbox/compare/v0.4.5...v0.5.0) (2026-10-03)

### Documentation

* **context:** record the dangling v0.4.5 tag and the preMajor bump policy (#78)
([83d3dc2](https://github.com/Archont561/pixi-sandbox/commit/83d3dc2e5e12df043ddf46d5d76e9240de69be21)),
closes [#78](https://github.com/Archont561/pixi-sandbox/issues/78)
[#77](https://github.com/Archont561/pixi-sandbox/issues/77)
* **context:** close the self-update core session (#77)
([dc7ec03](https://github.com/Archont561/pixi-sandbox/commit/dc7ec033017dbc94715fd120a496a13f0f6a4dc3)),
closes [#77](https://github.com/Archont561/pixi-sandbox/issues/77)
[#76](https://github.com/Archont561/pixi-sandbox/issues/76)
[#2](https://github.com/Archont561/pixi-sandbox/issues/2)

### [v0.4.5](https://github.com/Archont561/pixi-sandbox/compare/v0.4.4...v0.4.5) (2026-10-02)

#### Features

* **self-update:** verify and replace a standalone binary from a release (#76)
([79e4f83](https://github.com/Archont561/pixi-sandbox/commit/79e4f838ba9384cab87b2a16c0898c63d908dcf6)),
closes [#76](https://github.com/Archont561/pixi-sandbox/issues/76)
[#3](https://github.com/Archont561/pixi-sandbox/issues/3)

### [v0.4.4](https://github.com/Archont561/pixi-sandbox/compare/v0.4.3...v0.4.4) (2026-10-02)

#### Fixes

* **publish:** use source binary throughout owner workflow (#74)
([15b82c7](https://github.com/Archont561/pixi-sandbox/commit/15b82c755652cca3f333009d1036090487f2d67b)),
closes [#74](https://github.com/Archont561/pixi-sandbox/issues/74)
* **init:** stop appending the unusable namespace root to consumer channels; publish to the
archont561 ecosystem channel (#73)
([47f77c6](https://github.com/Archont561/pixi-sandbox/commit/47f77c6eef9817688fe97a1dc2a3c80ff5c855b5)),
closes [#73](https://github.com/Archont561/pixi-sandbox/issues/73)
[#71](https://github.com/Archont561/pixi-sandbox/issues/71)
[#72](https://github.com/Archont561/pixi-sandbox/issues/72)
[#1](https://github.com/Archont561/pixi-sandbox/issues/1)
[#3](https://github.com/Archont561/pixi-sandbox/issues/3)
[#4](https://github.com/Archont561/pixi-sandbox/issues/4)
[#5](https://github.com/Archont561/pixi-sandbox/issues/5)
[#2](https://github.com/Archont561/pixi-sandbox/issues/2)
[#6](https://github.com/Archont561/pixi-sandbox/issues/6)
[#7](https://github.com/Archont561/pixi-sandbox/issues/7)
[#72](https://github.com/Archont561/pixi-sandbox/issues/72)
[#73](https://github.com/Archont561/pixi-sandbox/issues/73)
[#72](https://github.com/Archont561/pixi-sandbox/issues/72)
[#72](https://github.com/Archont561/pixi-sandbox/issues/72)
[#73](https://github.com/Archont561/pixi-sandbox/issues/73)

#### Documentation

* **context:** hand off v1 self-update session (#75)
([41dc22f](https://github.com/Archont561/pixi-sandbox/commit/41dc22f24fec06d3ee4b0519a3048d2e8f2f4178)),
closes [#75](https://github.com/Archont561/pixi-sandbox/issues/75)

### [v0.4.3](https://github.com/Archont561/pixi-sandbox/compare/v0.4.2...v0.4.3) (2026-10-02)

#### Fixes

* enforce xtask Git boundary and clean docs warnings (#66)
([5fd728a](https://github.com/Archont561/pixi-sandbox/commit/5fd728abe4fc45f54bd3bb643c929556552e0217)),
closes [#66](https://github.com/Archont561/pixi-sandbox/issues/66)
* **ci:** repack the sandbox transport when a release lands (#65)
([482ab51](https://github.com/Archont561/pixi-sandbox/commit/482ab51ec83ff6e5fdd99eab7fcd2462ae3004f9)),
closes [#65](https://github.com/Archont561/pixi-sandbox/issues/65)

#### Documentation

* **backlog:** close the transport repack task (#67)
([098142a](https://github.com/Archont561/pixi-sandbox/commit/098142ae5a013932ac7e9a132705cd629dd2c16c)),
closes [#67](https://github.com/Archont561/pixi-sandbox/issues/67)

### [v0.4.2](https://github.com/Archont561/pixi-sandbox/compare/v0.4.1...v0.4.2) (2026-10-02)

#### Fixes

* **publish:** build a static source bootstrap (#63)
([b7ad517](https://github.com/Archont561/pixi-sandbox/commit/b7ad5171d45187af2657ec754b0798ae7a0f1c9c)),
closes [#63](https://github.com/Archont561/pixi-sandbox/issues/63)

#### Documentation

* **backlog:** close source-built publisher task (#64)
([dbe57a8](https://github.com/Archont561/pixi-sandbox/commit/dbe57a8c0bdda37aa01391a2d19a5134259c936e)),
closes [#64](https://github.com/Archont561/pixi-sandbox/issues/64)

### [v0.4.1](https://github.com/Archont561/pixi-sandbox/compare/v0.4.0...v0.4.1) (2026-10-02)

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
