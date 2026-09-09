# Prompt 01 — ARO-P0A Semantic AppProfile Cache

@GitHub tiếp tục trực tiếp repository `binhminhanh1235/agent-desktop-x`, không chỉ phân tích.

Working branch:

`feat/agent-runtime-optimization`

Initiative docs:

- `docs/agent-runtime-optimization-plan.md`
- `docs/agent-runtime-optimization-tracking.md`

Mục tiêu duy nhất của prompt này:

**ARO-P0A — Semantic AppProfile Cache**

Không bắt đầu P0B/P0C. Không merge vào `main` cho tới khi exact-head CI + CodeQL + Supply Chain đều PASS.

## Trước khi sửa

1. Re-check exact branch HEAD/tree.
2. Re-check current `main` HEAD/tree.
3. Xác minh baseline `main` gần nhất đã VERIFIED:
   - `72772258cca0471fed3eb8603eba0185eced55c2`
   - tree `5c5f9aa475064e56783cb9c7d46aea1b7060ee2b`
   - CI / CodeQL / Supply Chain / Release: PASS.
4. Đọc toàn bộ code hiện tại liên quan đến:
   - element/ref identity;
   - process/window generation;
   - tree/snapshot/walker/cache;
   - target resolution;
   - stale-ref handling;
   - platform adapter boundaries;
   - existing benchmarks.
5. Không đoán architecture từ tên file. Xác định call path thật trước khi thiết kế.

## Nguyên tắc bắt buộc

Mục tiêu là:

```text
DISCOVER ONCE
  -> CACHE SEMANTICS
  -> EXECUTE MANY
  -> VERIFY / REVALIDATE
```

Nhưng:

**Không cache raw element instance như durable identity.**

Phải tách rõ:

### Live reference cache

Short-lived, generation-bound:

- platform runtime id / handle;
- HWND/AX/UI element reference;
- current bounds/value;
- current process/window generation.

Live reference phải invalid khi process/window generation thay đổi.

### Semantic AppProfile cache

Có thể tồn tại lâu hơn:

- app identity/signature;
- window signature;
- automation/accessibility id;
- role/control type;
- normalized name;
- semantic ancestor path;
- supported actions/patterns;
- selector recipe;
- optional historical resolution metadata.

Bounds chỉ là fallback signal, không phải primary durable identity.

## Required implementation

### 1. Inventory + design seam

Trước tiên lập một dependency/call-path map ngắn trong PR hoặc docs cho:

```text
command/tool
  -> target resolution
  -> core ref identity
  -> platform adapter
  -> accessibility provider/tree
```

Chọn **một vertical slice nhỏ nhưng production-real** để đưa semantic cache vào trước. Không refactor toàn repo một lượt.

### 2. AppProfile model

Thiết kế typed Rust model tối thiểu cho:

- app identity;
- window signature;
- semantic selector;
- selector signals;
- cache generation/lifetime;
- resolution outcome;
- diagnostics.

Model không được chứa platform handle durable.

### 3. Semantic selector resolution

Resolution order nên ưu tiên signal mạnh trước, ví dụ:

```text
valid live ref
  -> exact stable accessibility/automation id
  -> semantic ancestor path
  -> role/control type + normalized name
  -> bounded fuzzy semantic candidate
  -> scoped live scan fallback
```

Không thêm vision trong P0A.

### 4. Scoped revalidation

Khi cached selector không còn exact match:

- không full-scan desktop ngay;
- revalidate trong app/window/subtree scope nhỏ nhất có thể;
- update semantic cache nếu candidate mới đủ tin cậy;
- nếu ambiguity cao thì refuse.

### 5. Safety

Bắt buộc giữ nguyên hoặc mạnh hơn các invariant hiện tại:

- stale process/window generation không được xem là live;
- mutating target ambiguity không được silently heal;
- không replay mutation nếu delivery state không chắc chắn;
- giữ nguyên outward error taxonomy trừ khi có migration/test rõ;
- giữ delivery semantics hiện có.

### 6. Diagnostics

Mỗi resolution nên có machine-readable diagnostics tối thiểu để benchmark/debug:

- `live_hit`
- `semantic_hit`
- `revalidated`
- `cache_miss`
- `ambiguous`
- `invalidated`
- fallback scope;
- signals used.

Không làm output CLI/MCP hiện tại phình lên mặc định nếu chưa cần. Có thể để diagnostics ở internal/test/opt-in seam.

### 7. Benchmark instrumentation

Thêm cách đo ít nhất:

- provider/tree reads;
- target-resolution calls;
- cold lookup;
- warm lookup.

Acceptance cần chứng minh warm stable lookup dùng ít provider/tree work hơn cold lookup.

Không claim phần trăm speedup nếu chưa benchmark thật.

## Required tests

Tối thiểu:

1. first lookup learns/produces semantic selector recipe.
2. repeated stable lookup takes warm path.
3. warm path performs fewer provider/tree reads than cold path.
4. element moves but semantic identity stays stable.
5. process restart invalidates live ref.
6. semantic selector can re-resolve after restart when target identity remains valid.
7. window recreation invalidates live generation.
8. duplicate same-name candidates refuse.
9. corrupted/mismatched cache fails closed.
10. bounds change alone does not break semantic identity.
11. existing stale-ref tests remain green.
12. existing delivery-semantics tests remain green.
13. Linux/macOS/Windows compile/contracts remain compatible.

## Storage scope for P0A

Ưu tiên **in-memory typed cache first** nếu đó là vertical slice nhỏ nhất đúng kiến trúc.

Không thêm SQLite/persistent disk storage chỉ để hoàn thành P0A trừ khi code hiện tại đã có persistence seam phù hợp.

Persistent AppProfile storage có thể đến sau khi model/lifetime đã được chứng minh.

## File-size / project rules

Tuân thủ toàn bộ source rules hiện có, bao gồm giới hạn file và comment/style checks. Không workaround test hoặc disable gate.

## Verification flow

Sau implementation:

1. cargo fmt / clippy / targeted tests.
2. chạy/re-check exact-head full CI.
3. re-check CodeQL.
4. re-check Supply Chain.
5. Nếu failure:
   - đọc đúng failed job/step/log;
   - sửa root cause;
   - tạo exact-head evidence mới.
6. Cập nhật:
   - `docs/agent-runtime-optimization-tracking.md`
   - `docs/agent-runtime-optimization-plan.md` nếu design thực tế khác plan.
7. Chỉ khi exact-head CI + CodeQL + Supply Chain PASS:
   - mở/hoàn thiện PR;
   - guarded merge với exact expected head SHA.
8. Sau merge:
   - verify exact new main HEAD/tree;
   - post-merge CI;
   - CodeQL;
   - Supply Chain;
   - Release.
9. Chỉ khi tất cả PASS mới đánh dấu:
   **ARO-P0A DONE / VERIFIED**.

## Deliverable cuối

Báo cáo:

- exact final branch head/tree;
- files changed;
- architecture seam được chọn;
- cold vs warm benchmark evidence;
- safety invariants;
- test evidence;
- CI/CodeQL/Supply Chain run IDs;
- PR;
- merge commit/tree;
- post-merge evidence;
- task tracking update;
- phần việc tiếp theo nhưng **không tự bắt đầu P0B**.
