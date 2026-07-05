--TEST--
FFI unsafe static methods fail closed by default
--EXTENSIONS--
ffi
--FILE--
<?php
FFI::cdef('int puts(const char *s);');
?>
--EXPECTF--
%s: runtime-diagnostic: %s"E_PHP_VM_UNSUPPORTED_FFI"%sFFI is disabled by default; unsafe FFI requires an explicit capability gate%s
%s: runtime_error: E_PHP_VM_UNSUPPORTED_FFI: FFI is disabled by default; unsafe FFI requires an explicit capability gate
