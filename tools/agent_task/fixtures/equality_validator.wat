(module
  (memory (export "memory") 3 3)
  (func (export "_start") (result i32)
    (local $len i32) (local $i i32)
    i32.const 0 i32.load i32.const 0x31545652 i32.ne
    if i32.const 1 return end
    i32.const 12 i32.load if i32.const 1 return end
    i32.const 4 i32.load local.tee $len
    i32.const 8 i32.load i32.ne if i32.const 1 return end
    (block $done (loop $compare
      local.get $i local.get $len i32.ge_u br_if $done
      i32.const 16 local.get $i i32.add i32.load8_u
      i32.const 16 local.get $len i32.add local.get $i i32.add i32.load8_u
      i32.ne if i32.const 1 return end
      local.get $i i32.const 1 i32.add local.set $i
      br $compare))
    i32.const 0))
