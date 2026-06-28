--TEST--
Generated reflection.extensions: ReflectionExtension exposes registered functions and classes
--DESCRIPTION--
module: reflection.extensions
generated timestamp: 20260628T000000Z
generator version: prompt21-reflection-v1
reason: ReflectionExtension MVP uses the builtin registry and extension owners for functions and classes.
--FILE--
<?php
$standard = new ReflectionExtension("standard");
echo $standard->getName(), "|";
$functions = $standard->getFunctions();
echo $functions["count"]->getName(), ":", $functions["count"]->getExtensionName(), "|";
$spl = new ReflectionExtension("spl");
$classes = $spl->getClassNames();
echo in_array("ArrayObject", $classes) ? "ArrayObject" : "missing";
?>
--EXPECT--
standard|count:standard|ArrayObject
