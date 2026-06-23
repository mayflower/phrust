<?php
class Phase7PropertyLoadBaseDto {
    public int $value = 5;
}

class Phase7PropertyLoadChildDto extends Phase7PropertyLoadBaseDto {
    public int $other = 1;
}

function phase7_property_load_base(Phase7PropertyLoadBaseDto $object): int {
    return $object->value;
}

echo phase7_property_load_base(new Phase7PropertyLoadBaseDto()), "|";
echo phase7_property_load_base(new Phase7PropertyLoadChildDto()), "\n";
