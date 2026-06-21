<?php
$a = ["outer" => ["inner" => 4]];
$a["outer"]["next"] = 8;
echo $a["outer"]["inner"], "|", $a["outer"]["next"], "\n";
