<?php
// runtime-semantics: expect=pass
$s = "abc";
echo $s[1], "|", $s["1"], "\n";

$s[1] = "Z";
echo $s, "\n";

$s[4] = "Q";
echo str_replace(" ", "_", $s), "\n";
