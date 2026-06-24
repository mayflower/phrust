<?php
// runtime-fixture: kind=valid expected_stdout="NULL|integer|boolean|string\n"
echo gettype(null), "|", gettype(7), "|", gettype(false), "|", gettype("x"), "\n";
