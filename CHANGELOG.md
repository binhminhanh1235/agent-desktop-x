# Changelog

## [0.9.0](https://github.com/binhminhanh1235/agent-desktop-x/compare/v0.8.5...v0.9.0) (2026-09-09)


### ⚠ BREAKING CHANGES

* `get --property text` no longer returns the same value as `--property value`. On roles whose value is not their readable content - every role but textfield, combobox, listbox, datefield and timefield - it now answers the accessible name. Callers depending on the previous behaviour should ask for `--property value` explicitly.
* retire the live Windows e2e lane and keep the desktop-free Windows gates
* the response envelope is version 2.3. launch returns { app, pid, process_instance, window? } instead of a bare window object, and an application that presents no window is ok:true with window omitted rather than WINDOW_NOT_FOUND. The C ABI is unchanged: it still writes one window and reports WINDOW_NOT_FOUND when there is none.
* ENVELOPE_VERSION is now 2.2. `data.complete` is present on every successful snapshot, and a snapshot that exhausts its budget returns `ok: true` with `complete: false` where it previously returned a TIMEOUT error. Callers that branched on TIMEOUT to detect an oversized tree must read `complete` instead.
* remove speculative Win32 private-file layer from core, add real Windows/Linux test lanes ([#106](https://github.com/binhminhanh1235/agent-desktop-x/issues/106))
* default-on auto-wait changes the timing of every previously-untouched ref-action call (bounded 5000 ms default; `--timeout-ms 0` restores single-shot). `ENVELOPE_VERSION` is now `2.1` (adds the `APP_UNRESPONSIVE` code and process state in error details). FFI ABI major is `3` (append-only struct evolution; `wait --event` is intentionally not exposed over FFI). The legacy string clipboard API is removed in favor of typed content. `key-down`/`key-up` fail closed until daemon-owned held input exists. `close-app` verifies termination and the osascript fallback path is removed. `--text` matching is subtree containment: `find --text X --first` returns the outermost matching container.
* the version command no longer accepts --json; it always emits the standard JSON envelope.
* chain execution deadlines now return TIMEOUT instead of ACTION_FAILED when the target app does not respond before the chain deadline.

* retire the live Windows e2e lane and keep the desktop-free Windows gates ([9bcb121](https://github.com/binhminhanh1235/agent-desktop-x/commit/9bcb121763a0b7c20a768c1165a44455395167e0))


### Features

* 10-step scroll chain, focus guards, enhanced click chain, bounds fix ([595ccb6](https://github.com/binhminhanh1235/agent-desktop-x/commit/595ccb6cc45554351ea3e30b95e4ca47bdf4e16b))
* add --wait-for selector polling flags ([#86](https://github.com/binhminhanh1235/agent-desktop-x/issues/86)) ([ce23278](https://github.com/binhminhanh1235/agent-desktop-x/commit/ce232787b50270d6f590e11bd2b16c7354de623d))
* add 19 new commands, AX-first rewrites, LOC compliance ([d3f7e03](https://github.com/binhminhanh1235/agent-desktop-x/commit/d3f7e03c67832c652a6125f61fbb7ab2f0801939))
* add 19 new commands, AX-first rewrites, LOC compliance ([eca04e8](https://github.com/binhminhanh1235/agent-desktop-x/commit/eca04e839288b121f6f41c6de525a8396d10654c))
* add agent-desktop skill for universal AI agent support ([ef45135](https://github.com/binhminhanh1235/agent-desktop-x/commit/ef45135087d09a7e065f65d9a0558d1e710cb8bf))
* add Claude Code skills for agent-desktop automation ([ad91cd3](https://github.com/binhminhanh1235/agent-desktop-x/commit/ad91cd32cf2de1c1c8dcda4c0dcae37f0022b4c6))
* add electron/web app compatibility for accessibility tree traversal ([a19c1b5](https://github.com/binhminhanh1235/agent-desktop-x/commit/a19c1b5132d3b71c5de58886ba51357ffc9bd1e8))
* add fallback chains for set-value, clear, focus, scroll-to, type and post-action state hints ([11f8da0](https://github.com/binhminhanh1235/agent-desktop-x/commit/11f8da06e84ed67b0e26dbc1946f7a7542e89dcd))
* add notification command types, adapter trait, and CLI wiring ([c5b05ba](https://github.com/binhminhanh1235/agent-desktop-x/commit/c5b05bab600aafa36c642f21837c44583b36459c))
* add notification management commands (macOS) ([b1fd368](https://github.com/binhminhanh1235/agent-desktop-x/commit/b1fd368f195640642adf011b75cca6ecb9e5acc3))
* add release automation with GitHub Releases and npm distribution ([18fc50c](https://github.com/binhminhanh1235/agent-desktop-x/commit/18fc50cca51f2ed10b6dfb5576602b6ce344bc95))
* add structural hints to splitter columns in snapshots ([48f8470](https://github.com/binhminhanh1235/agent-desktop-x/commit/48f8470948b4f636dfa6f4489e4cb6d9f520722c))
* add structured verbose logging across all layers ([c7316e8](https://github.com/binhminhanh1235/agent-desktop-x/commit/c7316e8b5160ab0e6ba554bcd502d4c47adf8b1a))
* add the action center notification adapter with verified mutations ([a6dd8fa](https://github.com/binhminhanh1235/agent-desktop-x/commit/a6dd8fa31c184bf7acab3d334e651a2511225b69))
* add the chromium dom-menu source to the menu detector with area 26 evidence ([58d42e9](https://github.com/binhminhanh1235/agent-desktop-x/commit/58d42e9cc881f92f65fafcbfe3ab3ce272056fb7))
* add the open-system-surface command through every registration point ([d4833db](https://github.com/binhminhanh1235/agent-desktop-x/commit/d4833db4decb836f63c3143a0bbbb7c070ebb514))
* add the shell-surface open and reach primitive for windows ([95c2ae6](https://github.com/binhminhanh1235/agent-desktop-x/commit/95c2ae618d44e0d9ac3b723db793aaf0ef8ed112))
* add trace viewer and replay artifacts ([e3e1872](https://github.com/binhminhanh1235/agent-desktop-x/commit/e3e1872ff3088f03f3e50e65ebc9e2b068a17b5f))
* add windows platform exploration probe corpus (phase 2.0) ([#111](https://github.com/binhminhanh1235/agent-desktop-x/issues/111)) ([5fd9543](https://github.com/binhminhanh1235/agent-desktop-x/commit/5fd9543a79859b1241dc6351fa87dfac7e935a9a))
* answer the next action while the last one's flourish plays ([988e73c](https://github.com/binhminhanh1235/agent-desktop-x/commit/988e73c0757014b8b45fb90375ca95f2833ca84a))
* AX-first right-click chain with inline context menu capture ([cddc5d3](https://github.com/binhminhanh1235/agent-desktop-x/commit/cddc5d3547f058a78f8b398fa982e39a1fcbf6b1))
* bundle skill docs and refactor --help for AI agents ([#36](https://github.com/binhminhanh1235/agent-desktop-x/issues/36)) ([b04d6f9](https://github.com/binhminhanh1235/agent-desktop-x/commit/b04d6f97317af67648890d2d3b5ead0d27c466c9))
* combine multi-agent cursors with core and macOS optimizations ([#171](https://github.com/binhminhanh1235/agent-desktop-x/issues/171)) ([922bb7d](https://github.com/binhminhanh1235/agent-desktop-x/commit/922bb7d5b59f2cc8cfaad9896c81fb8fed545bec))
* decide macOS delivery by observation and stop launch waiting on an uncaused event ([#125](https://github.com/binhminhanh1235/agent-desktop-x/issues/125)) ([298f1ff](https://github.com/binhminhanh1235/agent-desktop-x/commit/298f1ff21530958d765adf3834cdadccd4816a7a))
* drive Chromium apps through a verified DevTools endpoint from launch --cdp ([#129](https://github.com/binhminhanh1235/agent-desktop-x/issues/129)) ([366d348](https://github.com/binhminhanh1235/agent-desktop-x/commit/366d34803992d0595f31b19c8347bf7b26f5f277))
* drive the real windows adapter through the c abi against a staged window ([252339b](https://github.com/binhminhanh1235/agent-desktop-x/commit/252339bc24c9ff8d5b713a46f7b0622bcf54a207))
* emit the pressed state on Windows and match dangerous combos by superset ([a569216](https://github.com/binhminhanh1235/agent-desktop-x/commit/a56921661d07985670b35385a6a4ba3a28c2872c))
* fade the overlay, and show the card only when there is something to say ([d1b7dd4](https://github.com/binhminhanh1235/agent-desktop-x/commit/d1b7dd4adf7e5ce656b335a28a587d868bb00cf1))
* fail the npm gate when package and release matrix disagree ([ee3e75c](https://github.com/binhminhanh1235/agent-desktop-x/commit/ee3e75c1e096c4bc7d556cd711570f303bb8b0a0))
* **ffi:** Phase B and C — Python smoke harness, parity gates, build.rs codegen ([#77](https://github.com/binhminhanh1235/agent-desktop-x/issues/77)) ([9023f33](https://github.com/binhminhanh1235/agent-desktop-x/commit/9023f331b3981a41414ea0f92e2e5120a212e357))
* **ffi:** ship C-ABI cdylib with review hardening and release pipeline ([#26](https://github.com/binhminhanh1235/agent-desktop-x/issues/26)) ([3cffbd6](https://github.com/binhminhanh1235/agent-desktop-x/commit/3cffbd67f6b27f42001643bef9fd2530cb7f9003))
* get --property text answers what a person reads on the control ([dd61247](https://github.com/binhminhanh1235/agent-desktop-x/commit/dd6124721dd8a2d7e45dac61bbd6fd5376803cd7))
* give each agent its own cursor on Windows ([57c4fee](https://github.com/binhminhanh1235/agent-desktop-x/commit/57c4fee1508e7229ccb6fa13dd7b2d72130f908c))
* give find a traversal budget and a timeout that names the lever ([14a57ac](https://github.com/binhminhanh1235/agent-desktop-x/commit/14a57ac404b35e1e6309d330ae241b89766d1502))
* harden native automation and add cursor overlay ([#137](https://github.com/binhminhanh1235/agent-desktop-x/issues/137)) ([25a0087](https://github.com/binhminhanh1235/agent-desktop-x/commit/25a00877726d324c0ee64d84c8d989a5808a66d6))
* harden the Windows adapter and settle its cross-platform contract (sub-phase 2.15) ([de90dc0](https://github.com/binhminhanh1235/agent-desktop-x/commit/de90dc0b08b4f614f4ed3430dbac362d46f18f0d))
* implement --compact flag to collapse single-child unnamed nodes ([4a300c8](https://github.com/binhminhanh1235/agent-desktop-x/commit/4a300c8cb054462ca95fb5160e89e8fce661ec3b))
* implement Playwright-grade foundation contract ([3f32272](https://github.com/binhminhanh1235/agent-desktop-x/commit/3f322728b44548d2e22f4ee6ef4e6853af4e4550))
* install and run windows binaries through the npm channel ([8e3471c](https://github.com/binhminhanh1235/agent-desktop-x/commit/8e3471cf3a373666a9b81aa67933b22dab18309c))
* **macos,core:** harden adapter and core foundation with caller-controllable guardrails ([#82](https://github.com/binhminhanh1235/agent-desktop-x/issues/82)) ([94ce6c5](https://github.com/binhminhanh1235/agent-desktop-x/commit/94ce6c551fff4ffd2afc44ab6a9f655b4486da11))
* **macos:** add NC session RAII guard and notification adapter wiring ([0d55c21](https://github.com/binhminhanh1235/agent-desktop-x/commit/0d55c21de0f0ea6ea08ccb836e5527c9da513620))
* **macos:** implement dismiss and notification action commands ([53d697d](https://github.com/binhminhanh1235/agent-desktop-x/commit/53d697d52248c3fa06797787b1eb549ac2766533))
* **macos:** implement notification list via AX tree traversal ([53549a3](https://github.com/binhminhanh1235/agent-desktop-x/commit/53549a384eec67572ee05c31621b8b4174425ab3))
* make forgetting a retired overlay generation impossible ([950a075](https://github.com/binhminhanh1235/agent-desktop-x/commit/950a075ebb5d9541ba18911e0d38548b7bf19181))
* make sessions the first-class trace container ([35fa914](https://github.com/binhminhanh1235/agent-desktop-x/commit/35fa914b52bb0dffb0e630f9d9085789c3f81542))
* make windows a first-class distribution target ([e0842d0](https://github.com/binhminhanh1235/agent-desktop-x/commit/e0842d033ed4b8d4d8681b886a6213b7ad78858c))
* make windows a first-class distribution target ([#146](https://github.com/binhminhanh1235/agent-desktop-x/issues/146)) ([e0842d0](https://github.com/binhminhanh1235/agent-desktop-x/commit/e0842d033ed4b8d4d8681b886a6213b7ad78858c))
* merge upstream Windows adapter into fork main ([2277213](https://github.com/binhminhanh1235/agent-desktop-x/commit/2277213e0742c9f08e5461635578aeeb16c1aee6))
* open the Windows adapter phase with corrected platform facts ([4fa7661](https://github.com/binhminhanh1235/agent-desktop-x/commit/4fa766160869b4ecbfe43b49bb2688dce2b2e20e))
* Phase 1 foundation — workspace scaffold, core engine, macOS adapter, 31 commands ([a346f24](https://github.com/binhminhanh1235/agent-desktop-x/commit/a346f242c25dfad1c849e6d50f9ab25a42b462d9))
* probe area 26 shell surfaces and extend the capture redaction gate ([8095f4a](https://github.com/binhminhanh1235/agent-desktop-x/commit/8095f4aad177757264302e9fd3a4511fb76190ff))
* progressive skeleton traversal with ref-rooted drill-down ([#20](https://github.com/binhminhanh1235/agent-desktop-x/issues/20)) ([c17f2fa](https://github.com/binhminhanh1235/agent-desktop-x/commit/c17f2fae7abbbe2c914a050fa9e9be5fca9c6af0))
* publish enhanced reliability release ([8fbf904](https://github.com/binhminhanh1235/agent-desktop-x/commit/8fbf9049971c04dd57db4b7faf6352bc7d5e186f))
* rebuild the agent cursor overlay and fix seen-but-unactionable elements ([#145](https://github.com/binhminhanh1235/agent-desktop-x/issues/145)) ([5ec2504](https://github.com/binhminhanh1235/agent-desktop-x/commit/5ec2504bec0e9face5a69ef25c58bb1273772f2f))
* relocate the state root with AGENT_DESKTOP_HOME ([#135](https://github.com/binhminhanh1235/agent-desktop-x/issues/135)) ([a336b01](https://github.com/binhminhanh1235/agent-desktop-x/commit/a336b01e728893f379918b40157ab25f1c41fa80))
* render the cursor overlay on Windows ([7e31b1c](https://github.com/binhminhanh1235/agent-desktop-x/commit/7e31b1cd62c2c3956be0c97ec2ac88ed6406c9af))
* report cursor-overlay render truth and close the notification trace gap ([0f26d25](https://github.com/binhminhanh1235/agent-desktop-x/commit/0f26d25299bae057ce14c6eaa74225042670cc75))
* report hosted application identity through the frame host ([e6eae26](https://github.com/binhminhanh1235/agent-desktop-x/commit/e6eae26abd0bac8909d0d51102c750a05a3106fc))
* resolve shell surfaces by kind with no app and advertise the resolved set ([1c8938b](https://github.com/binhminhanh1235/agent-desktop-x/commit/1c8938b664334dde2cd036986dba204580f369a4))
* retire a renderer left behind by an earlier protocol generation ([03a2274](https://github.com/binhminhanh1235/agent-desktop-x/commit/03a2274409bae0c03eccdfee42637a6dc78642e0))
* retire an overlay renderer this build can no longer address ([8a0f102](https://github.com/binhminhanh1235/agent-desktop-x/commit/8a0f102e9258d04f6787a0c7a58098acb6503b44))
* return real per-process surface inventories on windows ([e88aa62](https://github.com/binhminhanh1235/agent-desktop-x/commit/e88aa627f0c027c2b788f34a707eab732d5a7867))
* scalable skill architecture with ClawHub auto-publishing ([#14](https://github.com/binhminhanh1235/agent-desktop-x/issues/14)) ([9766520](https://github.com/binhminhanh1235/agent-desktop-x/commit/97665203a464e605bc9b156ec90029c5909399be))
* ship and serve the windows skill package with embedding coverage ([6fd6e10](https://github.com/binhminhanh1235/agent-desktop-x/commit/6fd6e1041be9b542e11ac89c5688a0bdcf560317))
* ship the windows ffi import library and arm64 cdylib in releases ([cc634d7](https://github.com/binhminhanh1235/agent-desktop-x/commit/cc634d7878a0b4e4d64de7ab1bb2a5a64cdc04aa))
* smart AX-first click chain + macOS crate restructure ([4616c8f](https://github.com/binhminhanh1235/agent-desktop-x/commit/4616c8f65f974505b0eedb5485c865d3b905342b))
* surface-targeted snapshot, menu wait, list-surfaces command ([39178b2](https://github.com/binhminhanh1235/agent-desktop-x/commit/39178b291602d192de97aa0150c261db1dcc7ca6))
* the cursor overlay renders on Windows ([3152e85](https://github.com/binhminhanh1235/agent-desktop-x/commit/3152e85f9c5906589fb059e0cd7729f6116d92ba))
* uia element wrapper & tree walk (sub-phase 2.2) ([#114](https://github.com/binhminhanh1235/agent-desktop-x/issues/114)) ([41fc178](https://github.com/binhminhanh1235/agent-desktop-x/commit/41fc178e5efb4e7f7865de96b92178ddaa69d965))
* windows actionability and occlusion (sub-phase 2.6) ([#121](https://github.com/binhminhanh1235/agent-desktop-x/issues/121)) ([85f3b6e](https://github.com/binhminhanh1235/agent-desktop-x/commit/85f3b6e76a70bd8b3c0a8f41f8a846a03d0e0572))
* windows capture and clipboard (sub-phase 2.10) ([#126](https://github.com/binhminhanh1235/agent-desktop-x/issues/126)) ([c232035](https://github.com/binhminhanh1235/agent-desktop-x/commit/c23203586e6d8bce36e201504b8a4ebd772002d8))
* windows fixture app and live e2e harness (sub-phase 2.12) ([#132](https://github.com/binhminhanh1235/agent-desktop-x/issues/132)) ([69196c6](https://github.com/binhminhanh1235/agent-desktop-x/commit/69196c66a29e8ea76b92ff96c0620c8ef8578f9b))
* windows input synthesis (sub-phase 2.8) ([#123](https://github.com/binhminhanh1235/agent-desktop-x/issues/123)) ([4f7bad3](https://github.com/binhminhanh1235/agent-desktop-x/commit/4f7bad337c167486ada6f45bacfb0ee6c78a8e28))
* windows observation read path — snapshot, inventories, chromium settle, drill-down ([#119](https://github.com/binhminhanh1235/agent-desktop-x/issues/119)) ([bbad772](https://github.com/binhminhanh1235/agent-desktop-x/commit/bbad7721fbd949bdb28159c01d4da81e5192585b))
* windows resolution and live locator (sub-phase 2.5) ([#120](https://github.com/binhminhanh1235/agent-desktop-x/issues/120)) ([adf2c36](https://github.com/binhminhanh1235/agent-desktop-x/commit/adf2c36bdec7f9a6a9a9f4043a72e1738b1e3d39))
* windows semantic action tier (sub-phase 2.7) ([#122](https://github.com/binhminhanh1235/agent-desktop-x/issues/122)) ([5ed5633](https://github.com/binhminhanh1235/agent-desktop-x/commit/5ed56337ac7079301e2565038c32e5d1e37bdf4a))
* windows shell surfaces and notifications (sub-phase 2.14) ([a57b3fb](https://github.com/binhminhanh1235/agent-desktop-x/commit/a57b3fbae52fb17782f8d39f12939089f075eaee))
* windows signals and wait parity (sub-phase 2.11) ([#131](https://github.com/binhminhanh1235/agent-desktop-x/issues/131)) ([e612867](https://github.com/binhminhanh1235/agent-desktop-x/commit/e612867cf0e31fa52f1636609071df628c30a1f5))
* windows system lifecycle (sub-phase 2.9) ([#124](https://github.com/binhminhanh1235/agent-desktop-x/issues/124)) ([d4ddb55](https://github.com/binhminhanh1235/agent-desktop-x/commit/d4ddb5531ddacdd958bdf0c02420b40947e538d5))
* windows toolchain, ci & com bootstrap (sub-phase 2.1) ([#112](https://github.com/binhminhanh1235/agent-desktop-x/issues/112)) ([18daaa8](https://github.com/binhminhanh1235/agent-desktop-x/commit/18daaa8215e03c4e04fc7276236169f7560c27e1))
* windows vocabulary — roles, states, native_id and name evidence ([#115](https://github.com/binhminhanh1235/agent-desktop-x/issues/115)) ([8f24f04](https://github.com/binhminhanh1235/agent-desktop-x/commit/8f24f04f5a39cfdd680338e68c4643edd789d07f))


### Bug Fixes

* add clawhub login step before sync in CI ([208af12](https://github.com/binhminhanh1235/agent-desktop-x/commit/208af12459fea2255e1c80b8cdc9ac420316d769))
* add dwell time before drag release for drop target recognition ([2a52d62](https://github.com/binhminhanh1235/agent-desktop-x/commit/2a52d62106699b36f62f9af83895f2264b80efb1))
* add menubar surface, fix press --app crash and modifier mapping ([a231962](https://github.com/binhminhanh1235/agent-desktop-x/commit/a2319623b4d1d2b6b2f6e1a4ab9a8b8cbbfd02eb))
* address code review findings (double-free, CF leaks, injection) ([2f495ff](https://github.com/binhminhanh1235/agent-desktop-x/commit/2f495ffb69be67f3136b076534e078cc31b005c2))
* align error codes with spec (APP_NOT_FOUND, PERM_DENIED) and add -i shorthand ([6dc567a](https://github.com/binhminhanh1235/agent-desktop-x/commit/6dc567a4aedff15cf82a82601089cb0b87da4e26))
* ancestor-path cycle detection + CGEvent click fallback ([198d7d7](https://github.com/binhminhanh1235/agent-desktop-x/commit/198d7d7d27167044a448b6616fa5c9c0554321bf))
* answer a ref action terminally when its owning process is gone ([f60422a](https://github.com/binhminhanh1235/agent-desktop-x/commit/f60422a2e709302bcd6b4f3d68b13300aa30ce98))
* answer an overlay control before animating it ([0e20447](https://github.com/binhminhanh1235/agent-desktop-x/commit/0e20447fbc8bbe957edf1630b389cc5da35fac3d))
* bound the artifact lock wait by the capture's own budget ([0822e0e](https://github.com/binhminhanh1235/agent-desktop-x/commit/0822e0e9f31313210174bec3147f276b2209b4e8))
* catch a disappearance that happened after the wait started ([fe63031](https://github.com/binhminhanh1235/agent-desktop-x/commit/fe6303141f4eeb8c6060f40f3d90d7afaaf291cf))
* check SelectObject, stop a test from silently skipping, and record that tray clicks land ([dba944e](https://github.com/binhminhanh1235/agent-desktop-x/commit/dba944e8eea2dbe2262cd22248607336fc2b5a48))
* check the pipe server is our binary, not merely our user ([bc6ade8](https://github.com/binhminhanh1235/agent-desktop-x/commit/bc6ade80684de5429d30891e2b14cb586d6def07))
* clear the state-root override in test isolation instead of pinning it ([b22f941](https://github.com/binhminhanh1235/agent-desktop-x/commit/b22f94157d31f5b9b61caf5a15756d39adc4a899))
* clear the two CI failures this branch is responsible for ([8659c9c](https://github.com/binhminhanh1235/agent-desktop-x/commit/8659c9c6d5aafc47b97d10a3e174c814f99882ad))
* close five checks that could not distinguish success from failure ([9c8d2d6](https://github.com/binhminhanh1235/agent-desktop-x/commit/9c8d2d64081240ab20b3853aa182ae0f56bff45f))
* close four defects the full-branch review found in the read and write paths ([2ca493c](https://github.com/binhminhanh1235/agent-desktop-x/commit/2ca493cb1c6e99cf40b861c2b72657df115740fc))
* close the four ways the renderer could stop answering ([98520b6](https://github.com/binhminhanh1235/agent-desktop-x/commit/98520b647578c60537cb6bd49e30bad2ade6a753))
* close the release trust-model holes this branch's own dry-run seam opened ([ff9b3eb](https://github.com/binhminhanh1235/agent-desktop-x/commit/ff9b3eb0a546e289a1caaf907106738084f4cdb5))
* close the review findings on error classification, skip oracles, tray claims and the redaction scan set ([b626084](https://github.com/binhminhanh1235/agent-desktop-x/commit/b626084e2d5955af0da8d3664a649c5b2efbe395))
* complete Windows integration verification ([8698f3d](https://github.com/binhminhanh1235/agent-desktop-x/commit/8698f3d357e2c71194cf686824c257098b917dc5))
* correct GitHub Release download URL and simplify tag format ([8f66a93](https://github.com/binhminhanh1235/agent-desktop-x/commit/8f66a9346e02a751a83ac02313dcec2d9c81bde8))
* count only surface-shaped responses as the shell answering a raise ([b245fb3](https://github.com/binhminhanh1235/agent-desktop-x/commit/b245fb31ee6b24ac787ddc1d003f881035833032))
* detect open menus via AXMenuBarItem.AXSelected, not AXMenus attribute ([7f0d610](https://github.com/binhminhanh1235/agent-desktop-x/commit/7f0d6103d16969a0abfa84a62b6819dbd0d1cc8e))
* diagnose a foreign-shape raise across every shell host image ([315166f](https://github.com/binhminhanh1235/agent-desktop-x/commit/315166fbeda34bf85250247ecfd16d14f65b9d24))
* discard a monitor walk that did not finish ([11f2103](https://github.com/binhminhanh1235/agent-desktop-x/commit/11f2103a41cb3b80efbc7d0e66b71c93442b00c0))
* draw the pointer the right way up ([0016bd4](https://github.com/binhminhanh1235/agent-desktop-x/commit/0016bd4712790351b127fd7bdf98b5d3cf38aede))
* drop cargo caching from the privileged release jobs ([58dddc9](https://github.com/binhminhanh1235/agent-desktop-x/commit/58dddc99bbeeba592f0a87086676f6d091cb6eda))
* enforce the frame budgets where timings mean something ([19fa3dc](https://github.com/binhminhanh1235/agent-desktop-x/commit/19fa3dcc11eb9abe89b28ae1d5dbb970f0239ce3))
* enforce this gate as a ledger closure owner, and close the three rows it owns ([d9aedbe](https://github.com/binhminhanh1235/agent-desktop-x/commit/d9aedbe029f720882937e3f8f0a02211c739a85e))
* free the bitmap a failed SelectObject leaks, and pin the renderer match against a real window ([53c450a](https://github.com/binhminhanh1235/agent-desktop-x/commit/53c450a2cd04705da9217dd4a202e597fb14f2bb))
* gate SET_VALUE on a range control's read-only flag, and carry a chain timeout's steps ([e8523ce](https://github.com/binhminhanh1235/agent-desktop-x/commit/e8523cea8fd42f053e862dc2687896a21b4b39a4))
* give Escape the foreground it silently depends on ([c33cf73](https://github.com/binhminhanh1235/agent-desktop-x/commit/c33cf73592026a910b880eb2fb84caf0d5d65569))
* give Escape the foreground it silently depends on ([fcb7923](https://github.com/binhminhanh1235/agent-desktop-x/commit/fcb79230bf9ebb9a34f5551ab9459ee54b1a23be))
* give every Windows command the same --app identifier set ([cf358cf](https://github.com/binhminhanh1235/agent-desktop-x/commit/cf358cf3b5411a83157014f0ecc1ff38a6e426aa))
* give the Windows placeholder list the same words as the POSIX ones ([303663e](https://github.com/binhminhanh1235/agent-desktop-x/commit/303663e7eeabf168003c34e28890af2da1582660))
* handle null bounds in refmap and improve sidebar click resolution ([d4197e8](https://github.com/binhminhanh1235/agent-desktop-x/commit/d4197e8f6f6700f2f672d3e1e436ecf24cf82e01))
* harden macos ax window fallback ([3b266fd](https://github.com/binhminhanh1235/agent-desktop-x/commit/3b266fdf040bf83438f69a400b032fd12b8715c6))
* harden macos stale ref resolution ([#62](https://github.com/binhminhanh1235/agent-desktop-x/issues/62)) ([9f144c2](https://github.com/binhminhanh1235/agent-desktop-x/commit/9f144c2cafe3ba0ade479c1b87ecac6cd88adcef))
* include README and CHANGELOG in npm package ([084fc8c](https://github.com/binhminhanh1235/agent-desktop-x/commit/084fc8c960527c0d0654028794ec3c4fd2d970c4))
* keep a faulted host-window climb out of TIMEOUT ([541692d](https://github.com/binhminhanh1235/agent-desktop-x/commit/541692dbf690560193c112f12746d5bbd836887b))
* keep a nameless entry's bounds when they are its only identity ([f9d8fd2](https://github.com/binhminhanh1235/agent-desktop-x/commit/f9d8fd278f70800c1a8c6766e5bd002c3dbb63a2))
* keep serving after the first control, and exit when told to ([daf56c2](https://github.com/binhminhanh1235/agent-desktop-x/commit/daf56c20f75ab646bb58e0a7c2b1cacf0130b30a))
* keep the geometry a skeleton anchor is offered on, so its drill ref resolves ([8243fc5](https://github.com/binhminhanh1235/agent-desktop-x/commit/8243fc56579168a1619926b116ff2b3aedc178c9))
* let a faulted menu probe cost only the menu ([788f88c](https://github.com/binhminhanh1235/agent-desktop-x/commit/788f88c0d616a25f691ceee7eaa86b59cce00bbe))
* let a menu with no tool window still be located ([eff3d9b](https://github.com/binhminhanh1235/agent-desktop-x/commit/eff3d9bba4b52e17ff25a901f161d73fb74cede6))
* let a permanent refusal outrank a transient gap in the actionability verdict ([2edf9b4](https://github.com/binhminhanh1235/agent-desktop-x/commit/2edf9b4e228b42d2f1c336d1b09a0454671af76e))
* let the menu surface root what the menu wait fires on ([09caf52](https://github.com/binhminhanh1235/agent-desktop-x/commit/09caf52a6364f85df0d980a134da77a47e0432bf))
* **macos:** guard CFArray casts with type-ID check (fixes Mail.app crash) ([#50](https://github.com/binhminhanh1235/agent-desktop-x/issues/50)) ([c02cb5e](https://github.com/binhminhanh1235/agent-desktop-x/commit/c02cb5ecb7314f053d63937437e5f5ba48de3209))
* **macos:** harden retained_handle null guard against release-only CFRetain(null) ([#80](https://github.com/binhminhanh1235/agent-desktop-x/issues/80)) ([a708fa0](https://github.com/binhminhanh1235/agent-desktop-x/commit/a708fa03326a03dbc82acc7e41bd3ec262a05248))
* **macos:** remove AXPress from dismiss action list ([27ef4f3](https://github.com/binhminhanh1235/agent-desktop-x/commit/27ef4f34c038c26f5852bf1f6026762a98d0df0a))
* **macos:** restore frontmost app after notification center interaction ([3881bc8](https://github.com/binhminhanh1235/agent-desktop-x/commit/3881bc82bdb5a4bb7689f1e1e2237bb745e60c21))
* **macos:** use pgrep and async osascript for NC lifecycle ([9797585](https://github.com/binhminhanh1235/agent-desktop-x/commit/979758538fca0145e9245641fb03a0769eed68de))
* make a filtered dismiss-all target identities, not captured handles ([7675f80](https://github.com/binhminhanh1235/agent-desktop-x/commit/7675f803acfbe59e4050db85d4eb80a13bc76726))
* make all 30 commands work end-to-end on macOS ([1d98ab8](https://github.com/binhminhanh1235/agent-desktop-x/commit/1d98ab828ce5bcb39e212548ae2f2a052e67aac9))
* make the dry run reachable before the workflow lands on main ([ecd8af9](https://github.com/binhminhanh1235/agent-desktop-x/commit/ecd8af953f37ed2dd935fe35a2e5987dab0da7ec))
* make the wired gate executable, and stop a test pinning the masking that was removed ([627ca3c](https://github.com/binhminhanh1235/agent-desktop-x/commit/627ca3c6526da102fe6dfff530110ed8158fbd56))
* make two error envelopes name a recovery the caller can actually perform ([d6ad36c](https://github.com/binhminhanh1235/agent-desktop-x/commit/d6ad36cf4dc4988f674f0f1a418faec4fd5485a6))
* move the release dry run out of the privileged workflow ([09758ee](https://github.com/binhminhanh1235/agent-desktop-x/commit/09758ee3033f93132f6e9f4ca6b5979795f9c2e4))
* notice a session that switched its overlay off ([8b1f4d6](https://github.com/binhminhanh1235/agent-desktop-x/commit/8b1f4d6b12799e4cb708c2f5b2d6317d46be639c))
* point the topology tests at a path every platform can resolve ([d2edf11](https://github.com/binhminhanh1235/agent-desktop-x/commit/d2edf11e1f117f4db129759dabc0cb133c9700b0))
* reach the tray overflow surface through the taskbar automation tree ([9ccec88](https://github.com/binhminhanh1235/agent-desktop-x/commit/9ccec88ae39c95b0716deb8131d1dd45284527d5))
* read a provider's integer answer to a boolean property, and a climb's fault as a fault ([b355268](https://github.com/binhminhanh1235/agent-desktop-x/commit/b355268fff58b74195094bf99067f5f8f3a43b4b))
* read a token's user through one aligned path ([074ba46](https://github.com/binhminhanh1235/agent-desktop-x/commit/074ba46079d082b85d63bb8f33773ee1698c1ae4))
* read the session manifest from where it is actually written ([82edb01](https://github.com/binhminhanh1235/agent-desktop-x/commit/82edb01700b65c9807c804279442b44a6fbcdab9))
* read the source the way the checkout stores it ([4bd18d0](https://github.com/binhminhanh1235/agent-desktop-x/commit/4bd18d099c24bc193331c1a9b45032ca015dcf1e))
* record declined shell raises honestly in probes and tests ([367b385](https://github.com/binhminhanh1235/agent-desktop-x/commit/367b385c828b232063106faf70f228331c50553d))
* refuse a clipboard operation while a previous read still holds the clipboard open ([43da175](https://github.com/binhminhanh1235/agent-desktop-x/commit/43da175de3bb1a70374866cabaef52b0bd96009f))
* refuse a foreign-shape raise by observation instead of burning the deadline ([8fdfdfb](https://github.com/binhminhanh1235/agent-desktop-x/commit/8fdfdfb0db79fb4c4abe937983211e06ab2d2244))
* release the interaction lease by closing it, not by unlocking it ([c241dd1](https://github.com/binhminhanh1235/agent-desktop-x/commit/c241dd1f6213677683070d5ccfc6bca5385214d8))
* remove AXShowDefaultUI from activation chain, fix child walk ([74242f5](https://github.com/binhminhanh1235/agent-desktop-x/commit/74242f5040af9c46c98a3f5232dc7567538c28e1))
* repair corrupted Cargo.lock checksum ([881b354](https://github.com/binhminhanh1235/agent-desktop-x/commit/881b3544d8e34eb8f1eb5dea80dd83e5f5435afc))
* report the surfaces a process does have when one of its windows will not answer ([eda5222](https://github.com/binhminhanh1235/agent-desktop-x/commit/eda5222eb8a3eabd619c26934a339ebe8b24de57))
* resolve all 47 code review findings from Phase 1 audit ([218503a](https://github.com/binhminhanh1235/agent-desktop-x/commit/218503a7ebacacd4fbc6b388a6cf5e3bb86af039))
* resolve fullscreen AX tree retrieval returning ref_count: 0 ([11d01e1](https://github.com/binhminhanh1235/agent-desktop-x/commit/11d01e1c9ff38a8f91149fb5d8d20e6672e3454c))
* resolve fullscreen AX tree retrieval returning ref_count: 0 ([a52b7c7](https://github.com/binhminhanh1235/agent-desktop-x/commit/a52b7c704a4d13fdc0d6b72f4080bb0fc118be64))
* resolve tar to the in-box archiver so a Windows install can unpack its own release ([ebb8c63](https://github.com/binhminhanh1235/agent-desktop-x/commit/ebb8c6366589bc4c09d23b7d50a08289c0d8013d))
* resolve the system tray to the toolbar that actually holds the icons ([2f4aaf0](https://github.com/binhminhanh1235/agent-desktop-x/commit/2f4aaf03fa9917bc5169e73c0eb99b7a0eef7ae8))
* restore observed tree inspection helpers for Windows ([3217a83](https://github.com/binhminhanh1235/agent-desktop-x/commit/3217a837a744cfa1511cbfb45262461c52137135))
* return observed trees and stop demanding renderer activation from shallow walks ([#117](https://github.com/binhminhanh1235/agent-desktop-x/issues/117)) ([32175e4](https://github.com/binhminhanh1235/agent-desktop-x/commit/32175e44c553b350c90c311560ac4d341be71632))
* **review:** apply review findings ([de719c7](https://github.com/binhminhanh1235/agent-desktop-x/commit/de719c7a9ceee0372fe9baa308182430f88b5cc6))
* **review:** bound the arm64 capture leg to sessions that render pixels ([4cd26bf](https://github.com/binhminhanh1235/agent-desktop-x/commit/4cd26bff041a2dcd7911d60e76813b47f18a885e))
* **review:** drop the remaining block comment ([0c14caa](https://github.com/binhminhanh1235/agent-desktop-x/commit/0c14caade9408c641452147c54384f51fc18edd0))
* **review:** harden release checkouts and os-stub generation ([af5d6ef](https://github.com/binhminhanh1235/agent-desktop-x/commit/af5d6ef782ad4a083e4c770104520285f5105923))
* **review:** move capture skip rationale into the skip message ([d44beab](https://github.com/binhminhanh1235/agent-desktop-x/commit/d44beab8b1b15372ca50b40f357154c155daab62))
* **review:** move persist-credentials under checkout with ([42e4138](https://github.com/binhminhanh1235/agent-desktop-x/commit/42e4138707e8a2d9369408975b753f3e3d9aa885))
* right-click uses AXShowMenu; context menus detected via focused element ([2c9aee3](https://github.com/binhminhanh1235/agent-desktop-x/commit/2c9aee397912d6a903d9ef1e26c786697383ae95))
* run the overlay teardown tests where a desktop can actually be read ([d1f5e0a](https://github.com/binhminhanh1235/agent-desktop-x/commit/d1f5e0a3ea0d01edbdd5300880bb9477a4398886))
* scope the raise oracle to shell-host windows so suite noise cannot read as a response ([6be5d1c](https://github.com/binhminhanh1235/agent-desktop-x/commit/6be5d1cb0373e63fb30f374f83c9be7bca7ea48d))
* show skill install prompt on all success paths ([39b2bc6](https://github.com/binhminhanh1235/agent-desktop-x/commit/39b2bc63480890f7ed417b2c040eecf80c4628a0))
* show the label the caller asked for, not the greeting ([24b5c1d](https://github.com/binhminhanh1235/agent-desktop-x/commit/24b5c1dab182fe1d64cda621051a9a824de5bbe7))
* skip malformed inherited environment entries when launching ([d96d805](https://github.com/binhminhanh1235/agent-desktop-x/commit/d96d8058a6ed52bad1d6d2ab4fcc5813915fe909))
* spend the renderer start-up budget once, not once per attempt ([0bbafe5](https://github.com/binhminhanh1235/agent-desktop-x/commit/0bbafe5c56aaef438345c085ae00464a24bd357e))
* stabilize empty accessibility identity refs ([1fb5a7d](https://github.com/binhminhanh1235/agent-desktop-x/commit/1fb5a7d51eb798100b4d597c755fee1161e298bf))
* stop a batch test dumping its whole envelope into two assertion messages ([753f700](https://github.com/binhminhanh1235/agent-desktop-x/commit/753f700cc51f34f5c89ba507e2d1c0fa1a52a24b))
* stop a failed recovery from replacing the diagnosis it was recovering from ([2258e6a](https://github.com/binhminhanh1235/agent-desktop-x/commit/2258e6a1e3d5091fc447b934a79184d29205e18e))
* stop a faulted manifest read counting as an ended session ([e9bc7de](https://github.com/binhminhanh1235/agent-desktop-x/commit/e9bc7de05f491a129c978786c8b8be9dca19e9cd))
* stop a find test dumping a whole command response into its failure message ([0cc8653](https://github.com/binhminhanh1235/agent-desktop-x/commit/0cc86530a86b1d51e9920923354a8fc8bffa224c))
* stop a monitor whose info cannot be read vanishing from the list ([8eea437](https://github.com/binhminhanh1235/agent-desktop-x/commit/8eea437ca9843cf572d433b6015f92b6c0cb45b4))
* stop a window handle above Int64 aborting the whole e2e suite ([fa30fe6](https://github.com/binhminhanh1235/agent-desktop-x/commit/fa30fe68e8d682357cf53135dd79bddacc7301e2))
* stop the adopt path releasing a lease it did not take ([43be3c9](https://github.com/binhminhanh1235/agent-desktop-x/commit/43be3c929c3bf35edad75d0d3d15a332ae753655))
* stop the CLI hiding which argument is missing, and warn that PowerShell eats a bare ref ([c68e679](https://github.com/binhminhanh1235/agent-desktop-x/commit/c68e679131c029277887dc0c16b866e6779f93df))
* stop the notification read and its gates from reporting a fault as an absence ([1b03e8c](https://github.com/binhminhanh1235/agent-desktop-x/commit/1b03e8c0f1b88c60087ef48319e38884b53ec34b))
* stop the state root refusing its own user when elevation changes ([07ccbdd](https://github.com/binhminhanh1235/agent-desktop-x/commit/07ccbddc03f4fc7808fc8613577cfc5e3504f107))
* stop three more faults reading as absences ([1770d6e](https://github.com/binhminhanh1235/agent-desktop-x/commit/1770d6e5d56b846e6512fc4b288670c62017baaf))
* stop tracing spending the budget of the action it observes ([cf4f18b](https://github.com/binhminhanh1235/agent-desktop-x/commit/cf4f18b49bd1d2e166ada8b2bd2a09e17f5890f1))
* suppress dead_code lint on BatchCommand deserializer struct ([608d4aa](https://github.com/binhminhanh1235/agent-desktop-x/commit/608d4aaaa195b95626f17aa4bbca2d69609f14cc))
* synthesize a character key with the modifiers its layout requires ([f32967a](https://github.com/binhminhanh1235/agent-desktop-x/commit/f32967a7788d95049a61de2a0939adb0d04696ed))
* tell a caller whose snapshot is in the wrong namespace what is wrong ([c9559a8](https://github.com/binhminhanh1235/agent-desktop-x/commit/c9559a881f1a93fdc281a6bf88eb52e3bd1e321a))
* tell a faulted read apart from a genuine absence in two more places ([18e9a5c](https://github.com/binhminhanh1235/agent-desktop-x/commit/18e9a5cda9dff9ae0ca8cf286a18f0876d41cb76))
* update scroll recovery tests for context-aware ref actions ([1a41f76](https://github.com/binhminhanh1235/agent-desktop-x/commit/1a41f765715d68ea5495b65857e7b7f1b90ec857))
* update Windows envelope tests for context-aware actions ([e99a9ad](https://github.com/binhminhanh1235/agent-desktop-x/commit/e99a9adcefd69ffb4c10839182cb4e95b0860e7b))
* use macos-latest for both build targets ([91c7677](https://github.com/binhminhanh1235/agent-desktop-x/commit/91c76777cb7ee864b45e14d123c79c08f0c2d5b9))
* use simple release strategy for workspace version bumps ([0ab78dd](https://github.com/binhminhanh1235/agent-desktop-x/commit/0ab78dde0e1ff702db6c8b667784fa456245b26b))
* verify an already-installed binary before serving it ([c67e90e](https://github.com/binhminhanh1235/agent-desktop-x/commit/c67e90ecdcd78082df69cbc91fbdfbdd42d59b05))
* write the trace html export through the user-output primitive ([#113](https://github.com/binhminhanh1235/agent-desktop-x/issues/113)) ([00a4282](https://github.com/binhminhanh1235/agent-desktop-x/commit/00a4282f194dbb860859689c34db281bdc5e70ee))


### Performance

* measure the dead-process ref action against the merge-base and close two holes in the vehicle ([2159dd9](https://github.com/binhminhanh1235/agent-desktop-x/commit/2159dd91122a80332b9ec4e8553106bed90b82c7))
* outline an element without walking its interior ([bc3cb81](https://github.com/binhminhanh1235/agent-desktop-x/commit/bc3cb81d4dd675e4853ab7ce8f5635e78a771418))
* stop supersampling pixels that were never in doubt ([9ea6e03](https://github.com/binhminhanh1235/agent-desktop-x/commit/9ea6e030d5247d6277adc7d40d2c1391f1c7f45d))
* use curl for binary download in postinstall ([ebafb71](https://github.com/binhminhanh1235/agent-desktop-x/commit/ebafb71603f5f2b32af8ac5bf6c88df3d6012f70))


### Refactoring

* over-engineering audit cleanup ([#64](https://github.com/binhminhanh1235/agent-desktop-x/issues/64)) ([dbb2be6](https://github.com/binhminhanh1235/agent-desktop-x/commit/dbb2be639ecc1f979031818259f587f951086b3c))
* remove speculative Win32 private-file layer from core, add real Windows/Linux test lanes ([#106](https://github.com/binhminhanh1235/agent-desktop-x/issues/106)) ([8ad66b8](https://github.com/binhminhanh1235/agent-desktop-x/commit/8ad66b8f2115704eed56e59e2709c4eddf3cffac))
* unify command execution contracts ([1291a9c](https://github.com/binhminhanh1235/agent-desktop-x/commit/1291a9cdbf0566424d38da1eab397d6d4091c06c))

## [0.8.5](https://github.com/lahfir/agent-desktop/compare/v0.8.4...v0.8.5) (2026-09-06)


### Features

* combine multi-agent cursors with core and macOS optimizations ([#171](https://github.com/lahfir/agent-desktop/issues/171)) ([922bb7d](https://github.com/lahfir/agent-desktop/commit/922bb7d5b59f2cc8cfaad9896c81fb8fed545bec))

## [0.8.4](https://github.com/lahfir/agent-desktop/compare/v0.8.3...v0.8.4) (2026-08-28)


### Features

* rebuild the agent cursor overlay and fix seen-but-unactionable elements ([#145](https://github.com/lahfir/agent-desktop/issues/145)) ([5ec2504](https://github.com/lahfir/agent-desktop/commit/5ec2504bec0e9face5a69ef25c58bb1273772f2f))

## [0.8.3](https://github.com/lahfir/agent-desktop/compare/v0.8.2...v0.8.3) (2026-08-22)


### Features

* harden native automation and add cursor overlay ([#137](https://github.com/lahfir/agent-desktop/issues/137)) ([25a0087](https://github.com/lahfir/agent-desktop/commit/25a00877726d324c0ee64d84c8d989a5808a66d6))

## [0.8.2](https://github.com/lahfir/agent-desktop/compare/v0.8.1...v0.8.2) (2026-08-20)


### Features

* relocate the state root with AGENT_DESKTOP_HOME ([#135](https://github.com/lahfir/agent-desktop/issues/135)) ([a336b01](https://github.com/lahfir/agent-desktop/commit/a336b01e728893f379918b40157ab25f1c41fa80))

## [0.8.1](https://github.com/lahfir/agent-desktop/compare/v0.8.0...v0.8.1) (2026-08-14)


### Features

* drive Chromium apps through a verified DevTools endpoint from launch --cdp ([#129](https://github.com/lahfir/agent-desktop/issues/129)) ([366d348](https://github.com/lahfir/agent-desktop/commit/366d34803992d0595f31b19c8347bf7b26f5f277))

## [0.8.0](https://github.com/lahfir/agent-desktop/compare/v0.7.0...v0.8.0) (2026-08-13)


### ⚠ BREAKING CHANGES

* the response envelope is version 2.3. launch returns { app, pid, process_instance, window? } instead of a bare window object, and an application that presents no window is ok:true with window omitted rather than WINDOW_NOT_FOUND. The C ABI is unchanged: it still writes one window and reports WINDOW_NOT_FOUND when there is none.

### Features

* decide macOS delivery by observation and stop launch waiting on an uncaused event ([#125](https://github.com/lahfir/agent-desktop/issues/125)) ([298f1ff](https://github.com/lahfir/agent-desktop/commit/298f1ff21530958d765adf3834cdadccd4816a7a))

## [0.7.0](https://github.com/lahfir/agent-desktop/compare/v0.6.0...v0.7.0) (2026-08-02)


### ⚠ BREAKING CHANGES

* ENVELOPE_VERSION is now 2.2. `data.complete` is present on every successful snapshot, and a snapshot that exhausts its budget returns `ok: true` with `complete: false` where it previously returned a TIMEOUT error. Callers that branched on TIMEOUT to detect an oversized tree must read `complete` instead.

### Bug Fixes

* return observed trees and stop demanding renderer activation from shallow walks ([#117](https://github.com/lahfir/agent-desktop/issues/117)) ([32175e4](https://github.com/lahfir/agent-desktop/commit/32175e44c553b350c90c311560ac4d341be71632))

## [0.6.0](https://github.com/lahfir/agent-desktop/compare/v0.5.0...v0.6.0) (2026-07-26)


### ⚠ BREAKING CHANGES

* private artifacts on Windows are no longer written through ACL-hardened handles. That hardening guarded refmap, trace, and session files on a platform where no command can produce them, and it had never executed. It returns in Phase 2.1, built on Windows against a CI lane that runs it, under the constraints recorded in `docs/solutions/best-practices/never-ship-platform-code-that-ci-cannot-execute.md`.

### Refactoring

* remove speculative Win32 private-file layer from core, add real Windows/Linux test lanes ([#106](https://github.com/lahfir/agent-desktop/issues/106)) ([8ad66b8](https://github.com/lahfir/agent-desktop/commit/8ad66b8f2115704eed56e59e2709c4eddf3cffac))
  * deleted 1,062 LOC of Windows-only `unsafe` Win32 file I/O from `agent-desktop-core` and dropped the `windows-sys` dependency; Windows now uses the same portable `std::fs` path as every other non-unix target
  * this is a structural fix, not a downgrade: `std::fs::OpenOptions` defaults `share_mode` to `FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE`, precisely the flag the deleted code omitted and the cause of its sharing-violation failures
  * `cargo test` now runs on windows-latest and ubuntu-latest, so every platform-conditional branch in core is executed on each PR instead of only compiled

### Bug Fixes

* the new platform lanes exposed four pre-existing defects that were unreachable while only macOS ran tests:
  * trace files were opened append-only while `trace.rs` locks that handle; Windows `LockFileEx` rejects append-only handles, so all trace writing failed with `Access is denied`
  * `refs_lock` tests placed lock files directly in `/tmp`, which is root-owned and world-writable, so the private-parent check correctly refused them
  * a trace test interpolated a raw path into a JSON string literal, so Windows paths produced invalid escapes and the event was silently dropped instead of skipped
  * the default `SystemOps::permission_report` returned `Denied`, so CLI preflight reported `PERM_DENIED` on Windows and Linux when the adapter is simply unimplemented; it now reports `Unknown` and surfaces the honest `PLATFORM_NOT_SUPPORTED`, including over the FFI
* the pinned toolchain gained `clippy` and `rustfmt`, which `profile = "minimal"` had omitted

## [0.5.0](https://github.com/lahfir/agent-desktop/compare/v0.4.7...v0.5.0) (2026-07-20)


### ⚠ BREAKING CHANGES

* default-on auto-wait changes the timing of every previously-untouched ref-action call (bounded 5000 ms default; `--timeout-ms 0` restores single-shot). `ENVELOPE_VERSION` is now `2.1` (adds the `APP_UNRESPONSIVE` code and process state in error details). FFI ABI major is `3` (append-only struct evolution; `wait --event` is intentionally not exposed over FFI). The legacy string clipboard API is removed in favor of typed content. `key-down`/`key-up` fail closed until daemon-owned held input exists. `close-app` verifies termination and the osascript fallback path is removed. `--text` matching is subtree containment: `find --text X --first` returns the outermost matching container.

### Features

* implement Playwright-grade foundation contract ([3f32272](https://github.com/lahfir/agent-desktop/commit/3f322728b44548d2e22f4ee6ef4e6853af4e4550))
  * default-on auto-wait: every ref action waits for actionability (visible, enabled, stable, unoccluded) before dispatching, under a bounded budget
  * live `find` locator plus a serializable `LocatorQuery`, and honest `is --property visible`
  * three-way `hit_test` occlusion gate and core `scroll_into_view` before element actions
  * `list-displays` and honest `--screen` with per-display scale factor; `native_id` identity spine; window-id-first resolution
  * `ProcessState` classification with the `APP_UNRESPONSIVE` code (envelope `2.1`); typed `ActionStep` delivery tier
  * `LaunchOptions` (`--arg`/`--env`/`--cwd`/`--no-attach`); baseline-diff desktop signals via `wait --event`
  * typed clipboard (`Text`/`Image`/`FileUrls`); mouse modifier chords and a `mouse-wheel` primitive
  * capability-supertrait `PlatformAdapter` split with `not_supported()` defaults, so Windows and Linux inherit the contract

## [0.4.7](https://github.com/lahfir/agent-desktop/compare/v0.4.6...v0.4.7) (2026-07-02)


### Features

* add trace viewer and replay artifacts ([e3e1872](https://github.com/lahfir/agent-desktop/commit/e3e1872ff3088f03f3e50e65ebc9e2b068a17b5f))

## [0.4.6](https://github.com/lahfir/agent-desktop/compare/v0.4.5...v0.4.6) (2026-07-02)


### Features

* make sessions the first-class trace container ([35fa914](https://github.com/lahfir/agent-desktop/commit/35fa914b52bb0dffb0e630f9d9085789c3f81542))

## [0.4.5](https://github.com/lahfir/agent-desktop/compare/v0.4.4...v0.4.5) (2026-06-30)


### Features

* add --wait-for selector polling flags ([#86](https://github.com/lahfir/agent-desktop/issues/86)) ([ce23278](https://github.com/lahfir/agent-desktop/commit/ce232787b50270d6f590e11bd2b16c7354de623d))

## [0.4.4](https://github.com/lahfir/agent-desktop/compare/v0.4.3...v0.4.4) (2026-06-29)


### Features

* **macos,core:** harden adapter and core foundation with caller-controllable guardrails ([#82](https://github.com/lahfir/agent-desktop/issues/82)) ([94ce6c5](https://github.com/lahfir/agent-desktop/commit/94ce6c551fff4ffd2afc44ab6a9f655b4486da11))

## [0.4.3](https://github.com/lahfir/agent-desktop/compare/v0.4.2...v0.4.3) (2026-06-28)


### Bug Fixes

* **macos:** harden retained_handle null guard against release-only CFRetain(null) ([#80](https://github.com/lahfir/agent-desktop/issues/80)) ([a708fa0](https://github.com/lahfir/agent-desktop/commit/a708fa03326a03dbc82acc7e41bd3ec262a05248))

## [0.4.2](https://github.com/lahfir/agent-desktop/compare/v0.4.1...v0.4.2) (2026-06-27)


### Features

* **ffi:** Phase B and C — Python smoke harness, parity gates, build.rs codegen ([#77](https://github.com/lahfir/agent-desktop/issues/77)) ([9023f33](https://github.com/lahfir/agent-desktop/commit/9023f331b3981a41414ea0f92e2e5120a212e357))

## [0.4.1](https://github.com/lahfir/agent-desktop/compare/v0.4.0...v0.4.1) (2026-06-26)


### Features

* complete FFI C-ABI surface (Phase A): load-time ABI handshake, session-scoped adapter, JSON-envelope command entrypoints (version, status, snapshot, wait, execute-by-ref), an optional tracing log callback, and a unified error-envelope contract ([#67](https://github.com/lahfir/agent-desktop/issues/67))

## [0.4.0](https://github.com/lahfir/agent-desktop/compare/v0.3.1...v0.4.0) (2026-06-24)


### ⚠ BREAKING CHANGES

* the version command no longer accepts --json; it always emits the standard JSON envelope.

### Refactoring

* over-engineering audit cleanup ([#64](https://github.com/lahfir/agent-desktop/issues/64)) ([dbb2be6](https://github.com/lahfir/agent-desktop/commit/dbb2be639ecc1f979031818259f587f951086b3c))

## [0.3.1](https://github.com/lahfir/agent-desktop/compare/v0.3.0...v0.3.1) (2026-06-21)


### Bug Fixes

* harden macos stale ref resolution ([#62](https://github.com/lahfir/agent-desktop/issues/62)) ([9f144c2](https://github.com/lahfir/agent-desktop/commit/9f144c2cafe3ba0ade479c1b87ecac6cd88adcef))

## [0.3.0](https://github.com/lahfir/agent-desktop/compare/v0.2.3...v0.3.0) (2026-06-20)

### ⚠ BREAKING CHANGES

* **ffi:** `AD_POLICY_KIND_PHYSICAL` is now `AD_POLICY_KIND_HEADED` (discriminant `2` unchanged).
* **ffi:** `AdRefEntry` and `AdDragParams` include additional reliability metadata; C ABI consumers must zero-initialize and validate with `AD_REF_ENTRY_SIZE`, `AD_DRAG_PARAMS_SIZE`, and the `ad_*_size()` accessors.
* `close-app` graceful responses now return `{ "method": "graceful", "requested": true }` instead of claiming `closed: true` before the app has exited.

### Features

* add Playwright-grade ref reliability with strict late resolution, session-scoped snapshots, deterministic stale/ambiguous target handling, and skeleton drill-down preservation ([#54](https://github.com/lahfir/agent-desktop/pull/54)).
* add Playwright-style headed/headless interaction policy: accessibility-first dispatch remains default, while `--headed` enables focused physical fallbacks only when needed ([#54](https://github.com/lahfir/agent-desktop/pull/54)).
* add retrying wait predicates, JSONL traces with secret redaction, richer actionability reports, and shared CLI/FFI execution through the same resolver/actionability/dispatch ladder ([#54](https://github.com/lahfir/agent-desktop/pull/54)).

### Bug Fixes

* harden macOS ref actions across focus/window raising, disclosure expansion, scroll, drag, numeric set-value, menu detection, stale-ref recovery, protected-process errors, and close-app confirmation ([#54](https://github.com/lahfir/agent-desktop/pull/54)).
* fail closed on unsafe fallback ambiguity and stale refs, including symlink-safe refstore/latest-snapshot reads and deadline-bound resolver work ([#54](https://github.com/lahfir/agent-desktop/pull/54)).

### Performance

* reduce repeated live actionability reads and add deadline-bound resolver behavior, with large-app snapshot and per-command CLI wall-clock gates in the E2E suite ([#54](https://github.com/lahfir/agent-desktop/pull/54)).

### Tests

* add real-app E2E coverage for headless/headed ref actions, sessions, snapshots, traces, waits, skeleton drill-down, menus, surfaces, drag, expand/collapse, and performance checks ([#54](https://github.com/lahfir/agent-desktop/pull/54)).

## [0.2.3](https://github.com/lahfir/agent-desktop/compare/v0.2.2...v0.2.3) (2026-06-06)


### Bug Fixes

* harden macos ax window fallback ([3b266fd](https://github.com/lahfir/agent-desktop/commit/3b266fdf040bf83438f69a400b032fd12b8715c6))
* resolve fullscreen AX tree retrieval returning ref_count: 0 ([a52b7c7](https://github.com/lahfir/agent-desktop/commit/a52b7c704a4d13fdc0d6b72f4080bb0fc118be64))

## [0.2.2](https://github.com/lahfir/agent-desktop/compare/v0.2.1...v0.2.2) (2026-06-02)


### Bug Fixes

* **macos:** guard CFArray casts with type-ID check (fixes Mail.app crash) ([#50](https://github.com/lahfir/agent-desktop/issues/50)) ([c02cb5e](https://github.com/lahfir/agent-desktop/commit/c02cb5ecb7314f053d63937437e5f5ba48de3209))

## [0.2.1](https://github.com/lahfir/agent-desktop/compare/v0.2.0...v0.2.1) (2026-05-23)


### Bug Fixes

* stabilize empty accessibility identity refs ([1fb5a7d](https://github.com/lahfir/agent-desktop/commit/1fb5a7d51eb798100b4d597c755fee1161e298bf))

## [0.2.0](https://github.com/lahfir/agent-desktop/compare/v0.1.14...v0.2.0) (2026-05-20)


### ⚠ BREAKING CHANGES

* chain execution deadlines now return TIMEOUT instead of ACTION_FAILED when the target app does not respond before the chain deadline.

### Refactoring

* unify command execution contracts ([1291a9c](https://github.com/lahfir/agent-desktop/commit/1291a9cdbf0566424d38da1eab397d6d4091c06c))

## [0.1.14](https://github.com/lahfir/agent-desktop/compare/v0.1.13...v0.1.14) (2026-05-04)


### Features

* bundle skill docs and refactor --help for AI agents ([#36](https://github.com/lahfir/agent-desktop/issues/36)) ([b04d6f9](https://github.com/lahfir/agent-desktop/commit/b04d6f97317af67648890d2d3b5ead0d27c466c9))

## [0.1.13](https://github.com/lahfir/agent-desktop/compare/v0.1.12...v0.1.13) (2026-04-17)


### Features

* **ffi:** ship C-ABI cdylib with review fixes and release pipeline ([#26](https://github.com/lahfir/agent-desktop/issues/26)) ([3cffbd6](https://github.com/lahfir/agent-desktop/commit/3cffbd67f6b27f42001643bef9fd2530cb7f9003))

## [0.1.12](https://github.com/lahfir/agent-desktop/compare/v0.1.11...v0.1.12) (2026-04-16)


### Features

* progressive skeleton traversal with ref-rooted drill-down ([#20](https://github.com/lahfir/agent-desktop/issues/20)) ([c17f2fa](https://github.com/lahfir/agent-desktop/commit/c17f2fae7abbbe2c914a050fa9e9be5fca9c6af0))

## [0.1.11](https://github.com/lahfir/agent-desktop/compare/v0.1.10...v0.1.11) (2026-03-03)


### Bug Fixes

* show skill install prompt on all success paths ([39b2bc6](https://github.com/lahfir/agent-desktop/commit/39b2bc63480890f7ed417b2c040eecf80c4628a0))

## [0.1.10](https://github.com/lahfir/agent-desktop/compare/v0.1.9...v0.1.10) (2026-03-03)


### Bug Fixes

* add clawhub login step before sync in CI ([208af12](https://github.com/lahfir/agent-desktop/commit/208af12459fea2255e1c80b8cdc9ac420316d769))

## [0.1.9](https://github.com/lahfir/agent-desktop/compare/v0.1.8...v0.1.9) (2026-03-03)


### Features

* scalable skill architecture with ClawHub auto-publishing ([#14](https://github.com/lahfir/agent-desktop/issues/14)) ([9766520](https://github.com/lahfir/agent-desktop/commit/97665203a464e605bc9b156ec90029c5909399be))

## [0.1.8](https://github.com/lahfir/agent-desktop/compare/v0.1.7...v0.1.8) (2026-03-01)


### Features

* add electron/web app compatibility for accessibility tree traversal ([a19c1b5](https://github.com/lahfir/agent-desktop/commit/a19c1b5132d3b71c5de58886ba51357ffc9bd1e8))
* implement --compact flag to collapse single-child unnamed nodes ([4a300c8](https://github.com/lahfir/agent-desktop/commit/4a300c8cb054462ca95fb5160e89e8fce661ec3b))

## [0.1.7](https://github.com/lahfir/agent-desktop/compare/v0.1.6...v0.1.7) (2026-02-28)


### Features

* add notification command types, adapter trait, and CLI wiring ([c5b05ba](https://github.com/lahfir/agent-desktop/commit/c5b05bab600aafa36c642f21837c44583b36459c))
* add notification management commands (macOS) ([b1fd368](https://github.com/lahfir/agent-desktop/commit/b1fd368f195640642adf011b75cca6ecb9e5acc3))
* **macos:** add NC session RAII guard and notification adapter wiring ([0d55c21](https://github.com/lahfir/agent-desktop/commit/0d55c21de0f0ea6ea08ccb836e5527c9da513620))
* **macos:** implement dismiss and notification action commands ([53d697d](https://github.com/lahfir/agent-desktop/commit/53d697d52248c3fa06797787b1eb549ac2766533))
* **macos:** implement notification list via AX tree traversal ([53549a3](https://github.com/lahfir/agent-desktop/commit/53549a384eec67572ee05c31621b8b4174425ab3))


### Bug Fixes

* **macos:** remove AXPress from dismiss action list ([27ef4f3](https://github.com/lahfir/agent-desktop/commit/27ef4f34c038c26f5852bf1f6026762a98d0df0a))
* **macos:** restore frontmost app after notification center interaction ([3881bc8](https://github.com/lahfir/agent-desktop/commit/3881bc82bdb5a4bb7689f1e1e2237bb745e60c21))
* **macos:** use pgrep and async osascript for NC lifecycle ([9797585](https://github.com/lahfir/agent-desktop/commit/979758538fca0145e9245641fb03a0769eed68de))

## [0.1.6](https://github.com/lahfir/agent-desktop/compare/v0.1.5...v0.1.6) (2026-02-24)


### Bug Fixes

* handle null bounds in refmap and improve sidebar click resolution ([d4197e8](https://github.com/lahfir/agent-desktop/commit/d4197e8f6f6700f2f672d3e1e436ecf24cf82e01))

## [0.1.5](https://github.com/lahfir/agent-desktop/compare/v0.1.4...v0.1.5) (2026-02-23)


### Features

* add fallback chains for set-value, clear, focus, scroll-to, type and post-action state hints ([11f8da0](https://github.com/lahfir/agent-desktop/commit/11f8da06e84ed67b0e26dbc1946f7a7542e89dcd))
* add structured verbose logging across all layers ([c7316e8](https://github.com/lahfir/agent-desktop/commit/c7316e8b5160ab0e6ba554bcd502d4c47adf8b1a))


### Bug Fixes

* add dwell time before drag release for drop target recognition ([2a52d62](https://github.com/lahfir/agent-desktop/commit/2a52d62106699b36f62f9af83895f2264b80efb1))

## [0.1.4](https://github.com/lahfir/agent-desktop/compare/v0.1.3...v0.1.4) (2026-02-23)


### Features

* add agent-desktop skill for universal AI agent support ([ef45135](https://github.com/lahfir/agent-desktop/commit/ef45135087d09a7e065f65d9a0558d1e710cb8bf))
* add Claude Code skills for agent-desktop automation ([ad91cd3](https://github.com/lahfir/agent-desktop/commit/ad91cd32cf2de1c1c8dcda4c0dcae37f0022b4c6))

## [0.1.3](https://github.com/lahfir/agent-desktop/compare/v0.1.2...v0.1.3) (2026-02-23)


### Bug Fixes

* correct GitHub Release download URL and simplify tag format ([8f66a93](https://github.com/lahfir/agent-desktop/commit/8f66a9346e02a751a83ac02313dcec2d9c81bde8))
* include README and CHANGELOG in npm package ([084fc8c](https://github.com/lahfir/agent-desktop/commit/084fc8c960527c0d0654028794ec3c4fd2d970c4))


### Performance

* use curl for binary download in postinstall ([ebafb71](https://github.com/lahfir/agent-desktop/commit/ebafb71603f5f2b32af8ac5bf6c88df3d6012f70))

## [0.1.2](https://github.com/lahfir/agent-desktop/compare/agent-desktop-v0.1.1...agent-desktop-v0.1.2) (2026-02-23)


### Bug Fixes

* use macos-latest for both build targets ([91c7677](https://github.com/lahfir/agent-desktop/commit/91c76777cb7ee864b45e14d123c79c08f0c2d5b9))

## [0.1.1](https://github.com/lahfir/agent-desktop/compare/agent-desktop-v0.1.0...agent-desktop-v0.1.1) (2026-02-23)


### Features

* 10-step scroll chain, focus guards, enhanced click chain, bounds fix ([595ccb6](https://github.com/lahfir/agent-desktop/commit/595ccb6cc45554351ea3e30b95e4ca47bdf4e16b))
* add 19 new commands, AX-first rewrites, LOC compliance ([d3f7e03](https://github.com/lahfir/agent-desktop/commit/d3f7e03c67832c652a6125f61fbb7ab2f0801939))
* add 19 new commands, AX-first rewrites, LOC compliance ([eca04e8](https://github.com/lahfir/agent-desktop/commit/eca04e839288b121f6f41c6de525a8396d10654c))
* add release automation with GitHub Releases and npm distribution ([18fc50c](https://github.com/lahfir/agent-desktop/commit/18fc50cca51f2ed10b6dfb5576602b6ce344bc95))
* add structural hints to splitter columns in snapshots ([48f8470](https://github.com/lahfir/agent-desktop/commit/48f8470948b4f636dfa6f4489e4cb6d9f520722c))
* AX-first right-click chain with inline context menu capture ([cddc5d3](https://github.com/lahfir/agent-desktop/commit/cddc5d3547f058a78f8b398fa982e39a1fcbf6b1))
* Phase 1 foundation — workspace scaffold, core engine, macOS adapter, 31 commands ([a346f24](https://github.com/lahfir/agent-desktop/commit/a346f242c25dfad1c849e6d50f9ab25a42b462d9))
* smart AX-first click chain + macOS crate restructure ([4616c8f](https://github.com/lahfir/agent-desktop/commit/4616c8f65f974505b0eedb5485c865d3b905342b))
* surface-targeted snapshot, menu wait, list-surfaces command ([39178b2](https://github.com/lahfir/agent-desktop/commit/39178b291602d192de97aa0150c261db1dcc7ca6))


### Bug Fixes

* add menubar surface, fix press --app crash and modifier mapping ([a231962](https://github.com/lahfir/agent-desktop/commit/a2319623b4d1d2b6b2f6e1a4ab9a8b8cbbfd02eb))
* address code review findings (double-free, CF leaks, injection) ([2f495ff](https://github.com/lahfir/agent-desktop/commit/2f495ffb69be67f3136b076534e078cc31b005c2))
* align error codes with spec (APP_NOT_FOUND, PERM_DENIED) and add -i shorthand ([6dc567a](https://github.com/lahfir/agent-desktop/commit/6dc567a4aedff15cf82a82601089cb0b87da4e26))
* ancestor-path cycle detection + CGEvent click fallback ([198d7d7](https://github.com/lahfir/agent-desktop/commit/198d7d7d27167044a448b6616fa5c9c0554321bf))
* detect open menus via AXMenuBarItem.AXSelected, not AXMenus attribute ([7f0d610](https://github.com/lahfir/agent-desktop/commit/7f0d6103d16969a0abfa84a62b6819dbd0d1cc8e))
* make all 30 commands work end-to-end on macOS ([1d98ab8](https://github.com/lahfir/agent-desktop/commit/1d98ab828ce5bcb39e212548ae2f2a052e67aac9))
* remove AXShowDefaultUI from activation chain, fix child walk ([74242f5](https://github.com/lahfir/agent-desktop/commit/74242f5040af9c46c98a3f5232dc7567538c28e1))
* resolve all 47 code review findings from Phase 1 audit ([218503a](https://github.com/lahfir/agent-desktop/commit/218503a7ebacacd4fbc6b388a6cf5e3bb86af039))
* right-click uses AXShowMenu; context menus detected via focused element ([2c9aee3](https://github.com/lahfir/agent-desktop/commit/2c9aee397912d6a903d9ef1e26c786697383ae95))
* suppress dead_code lint on BatchCommand deserializer struct ([608d4aa](https://github.com/lahfir/agent-desktop/commit/608d4aaaa195b95626f17aa4bbca2d69609f14cc))
* use simple release strategy for workspace version bumps ([0ab78dd](https://github.com/lahfir/agent-desktop/commit/0ab78dde0e1ff702db6c8b667784fa456245b26b))
