--TEST--
Generated objects.enums: enum cases
--DESCRIPTION--
module: objects.enums
generated timestamp: 20260627T000000Z
generator version: phpt-objects-traits-enums-v1
reason: Prompt 2.5 enum cases baseline
--FILE--
<?php
enum ObjectCasesStatus {
    case Draft;
    case Ready;
}

foreach (ObjectCasesStatus::cases() as $case) {
    echo $case->name, "|";
}
echo "\n";
?>
--EXPECT--
Draft|Ready|
