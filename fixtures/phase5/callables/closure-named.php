<?php
$join = function ($first, $second, ...$rest) {
    echo $first, "|", $second, "|", $rest["third"];
};

$join(second: "S", first: "F", third: "R1");
