<?php
$packed = [1, 2, 3, 4];
$packedSum = 0;
foreach ($packed as $value) {
    $packedSum += $value;
}
$packedRead = 0;
for ($i = 0; $i < 12; $i++) {
    $packedRead += $packed[2];
}
echo "packed=", count($packed), ":", $packedSum, ":", $packedRead, "\n";

$mixedElements = [1, "2", 3];
$mixedText = "";
foreach ($mixedElements as $value) {
    $mixedText .= $value;
}
echo "mixed=", $mixedText, "\n";

$empty = [];
$emptySum = 0;
foreach ($empty as $value) {
    $emptySum += $value;
}
echo "empty=", $emptySum, "\n";

$keys = [-1 => "neg", "01" => "numstr", "name" => "str"];
echo "keys=", $keys[-1], ":", $keys["01"], ":", $keys["name"], "\n";

$cow = [1, 2];
$copy = $cow;
$copy[] = 3;
echo "cow=", count($cow), ":", count($copy), ":", $cow[1], "\n";

$refItems = [1, 2];
$alias =& $refItems[0];
$alias = 7;
echo "refs=", $refItems[0], ":", $refItems[1], "\n";

$mutation = [1, 2];
foreach ($mutation as $key => $value) {
    echo $value, ",";
    if ($key === 0) {
        $mutation[] = 3;
    }
}
echo "|", count($mutation), "\n";

$nested = [[1, 2], ["x" => 3]];
echo "nested=", $nested[0][1], ":", $nested[1]["x"], "\n";

class ArrayFastDto
{
    public $id;
    public $name;

    public function __construct($row)
    {
        $this->id = $row["id"];
        $this->name = $row["name"];
    }
}

$rows = [["id" => 1, "name" => "a"], ["id" => 2, "name" => "b"]];
$dtoTotal = 0;
$dtoNames = "";
foreach ($rows as $row) {
    $dto = new ArrayFastDto($row);
    $dtoTotal += $dto->id;
    $dtoNames .= $dto->name;
}
echo "dto=", $dtoTotal, ":", $dtoNames, "\n";

$ambiguous = [10, 20];
$ambiguousKey = 1;
$ambiguousSum = 0;
for ($i = 0; $i < 13; $i++) {
    if ($i === 12) {
        $ambiguousKey = "01";
        $ambiguous["01"] = 30;
    }
    $ambiguousSum += $ambiguous[$ambiguousKey];
}
echo "ambiguous=", $ambiguousSum, "\n";

$overflow = [9223372036854775807, 1];
$overflowSum = 0;
foreach ($overflow as $value) {
    $overflowSum += $value;
}
echo "overflow=", var_export($overflowSum, true), "\n";
