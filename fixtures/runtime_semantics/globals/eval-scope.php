<?php
// runtime-semantics: category=globals expect=pass
$x = 2;
eval('$x = $x + 5; $GLOBALS["from_eval"] = "ok";');
echo $x, ":", $GLOBALS["from_eval"], "\n";
