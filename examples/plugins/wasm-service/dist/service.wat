;; Compile this file to service.wasm with wat2wasm before installation.
;; The host links no WASI imports: the component has no ambient OS authority.
(module
  (func (export "run") (result i32)
    i32.const 0))
