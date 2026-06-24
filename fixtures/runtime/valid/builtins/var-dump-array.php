<?php
// runtime-fixture: kind=valid expected_stdout="array(2) {\n  [0]=>\n  int(1)\n  [1]=>\n  string(1) \"x\"\n}\n"
function dump_args(...$args)
{
    var_dump($args);
}

dump_args(1, "x");
