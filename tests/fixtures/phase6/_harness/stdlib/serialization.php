<?php
// phase6-diff: id=PHASE6_STDLIB_SERIALIZATION area=stdlib expect=pass
echo serialize(null), "\n";
echo serialize(true), "\n";
echo serialize(7), "\n";
echo serialize("hi"), "\n";
echo serialize([1, "x"]), "\n";
echo var_export(unserialize('a:2:{i:0;i:1;i:1;s:1:"x";}'), true), "\n";
