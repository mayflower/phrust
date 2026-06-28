--TEST--
Generated reflection.enums: ReflectionEnum exposes backed cases
--DESCRIPTION--
module: reflection.enums
generated timestamp: 20260628T000000Z
generator version: prompt21-reflection-v1
reason: ReflectionEnum MVP covers backed enum type metadata, case lists, and backed case values.
--FILE--
<?php
enum P21Status: string {
    case Ready = "ready";
    case Done = "done";
}

$enum = new ReflectionEnum(P21Status::class);
echo $enum->getName(), "|";
echo $enum->isBacked() ? "backed|" : "unit|";
echo $enum->getBackingType()->getName(), "|";
$cases = $enum->getCases();
echo count($cases), "|", get_class($cases[0]), ":", $cases[0]->getName(), ":", $cases[0]->getBackingValue();
?>
--EXPECT--
P21Status|backed|string|2|ReflectionEnumBackedCase:Ready:ready
