--TEST--
Generated reflection.parameters: ReflectionParameter reads internal generated arginfo
--DESCRIPTION--
module: reflection.parameters
generated timestamp: 20260628T000000Z
generator version: prompt21-reflection-v1
reason: ReflectionParameter must expose names, optionality, variadic/by-ref flags, and simple types from generated arginfo.
--FILE--
<?php
$sort = new ReflectionFunction("sort");
echo $sort->getName(), "|";
foreach ($sort->getParameters() as $param) {
    echo $param->getName(), ":";
    echo $param->isOptional() ? "optional:" : "required:";
    echo $param->isVariadic() ? "variadic:" : "fixed:";
    echo $param->isPassedByReference() ? "byref:" : "byval:";
    echo $param->hasType() ? $param->getType()->getName() : "none";
    echo "|";
}
echo "\n";

$printf = new ReflectionFunction("printf");
echo $printf->getName(), "|";
foreach ($printf->getParameters() as $param) {
    echo $param->getName(), ":";
    echo $param->isOptional() ? "optional:" : "required:";
    echo $param->isVariadic() ? "variadic:" : "fixed:";
    echo $param->isPassedByReference() ? "byref:" : "byval:";
    echo $param->hasType() ? $param->getType()->getName() : "none";
    echo "|";
}
?>
--EXPECT--
sort|array:required:fixed:byref:array|flags:optional:fixed:byval:int|
printf|format:required:fixed:byval:string|values:optional:variadic:byval:mixed|
