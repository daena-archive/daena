;; The bundled Timeline provider is intentionally a pure synchronous service.
;; It echoes the JSON request, which preserves the resolved date payload while
;; keeping the provider inside the same sandboxed ABI used by installed WASM.
(module
  (memory (export "memory") 5 5)
  (func (export "alloc") (param i32) (result i32)
    i32.const 16)
  (func (export "handle_json") (param i32 i32) (result i64)
    local.get 1
    i64.extend_i32_u
    i64.const 32
    i64.shl
    local.get 0
    i64.extend_i32_u
    i64.or))
