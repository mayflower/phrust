--TEST--
FFI disabled-by-default surface metadata
--EXTENSIONS--
ffi
--FILE--
<?php
echo extension_loaded('ffi') ? "loaded\n" : "missing\n";
foreach (['FFI', 'FFI\\CData', 'FFI\\CType', 'FFI\\Exception', 'FFI\\ParserException'] as $class) {
    echo class_exists($class) ? "$class class\n" : "$class missing\n";
}
$class = new ReflectionClass('FFI');
echo $class->getName(), "|", $class->getExtensionName(), "|", ($class->isInternal() ? "internal" : "user"), "\n";
foreach (['cdef', 'load', 'new', 'cast', 'typeof', 'addr', 'sizeof', 'alignof', 'memcpy', 'memcmp', 'memset', 'string', 'isNull', 'arrayType', 'free', 'scope', 'type'] as $method) {
    echo $class->hasMethod($method) ? "$method method\n" : "$method missing\n";
}
$method = $class->getMethod('cdef');
echo $method->getName(), "|", ($method->isStatic() ? "static" : "instance"), "|", $method->getNumberOfParameters(), "\n";
?>
--EXPECT--
loaded
FFI class
FFI\CData class
FFI\CType class
FFI\Exception class
FFI\ParserException class
FFI|ffi|internal
cdef method
load method
new method
cast method
typeof method
addr method
sizeof method
alignof method
memcpy method
memcmp method
memset method
string method
isNull method
arrayType method
free method
scope method
type method
cdef|static|2
