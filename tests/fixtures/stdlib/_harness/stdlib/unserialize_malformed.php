<?php
// stdlib-diff: id=STDLIB_UNSERIALIZE_MALFORMED_WARNING area=stdlib
echo var_export(unserialize('bad payload'), true), "\n";
