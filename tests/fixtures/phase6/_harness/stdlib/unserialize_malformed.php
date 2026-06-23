<?php
// phase6-diff: id=PHASE6_STDLIB_UNSERIALIZE_MALFORMED_WARNING area=stdlib
echo var_export(unserialize('bad payload'), true), "\n";
