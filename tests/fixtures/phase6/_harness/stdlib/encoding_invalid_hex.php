<?php
// phase6-diff: id=PHASE6_STDLIB_ENCODING_INVALID_HEX area=stdlib
echo var_export(hex2bin("f"), true), "\n";
echo var_export(hex2bin("zz"), true), "\n";
