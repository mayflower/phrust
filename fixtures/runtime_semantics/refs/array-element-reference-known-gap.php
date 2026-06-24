<?php
// runtime-semantics: expect=pass
$array = ["k" => 1];
$alias =& $array["k"];
$alias = 2;
echo $array["k"];
