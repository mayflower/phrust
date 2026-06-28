--TEST--
Generated reflection.functions: ReflectionFunction covers internal and userland metadata
--DESCRIPTION--
module: reflection.functions
generated timestamp: 20260628T000000Z
generator version: prompt21-reflection-v1
reason: ReflectionFunction must read generated arginfo for internal functions and IR metadata for userland functions.
--FILE--
<?php
$internal = new ReflectionFunction("count");
echo $internal->getName(), "|";
echo $internal->isInternal() ? "internal|" : "user|";
echo $internal->getNumberOfParameters(), ":", $internal->getNumberOfRequiredParameters(), "|";
echo $internal->getReturnType()->getName(), "|", $internal->getExtensionName(), "\n";

function p21_user(int $id, string ...$names): bool { return true; }
$user = new ReflectionFunction("p21_user");
echo $user->getName(), "|";
echo $user->isUserDefined() ? "user|" : "internal|";
echo $user->getNumberOfParameters(), ":", $user->getNumberOfRequiredParameters(), "|";
echo $user->getReturnType()->getName();
?>
--EXPECT--
count|internal|2:1|int|standard
p21_user|user|2:1|bool
