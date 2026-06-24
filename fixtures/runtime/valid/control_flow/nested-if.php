<?php
// runtime-fixture: kind=valid expected_stdout="nested\n"
$flag = true;
if ($flag) {
    if (1 < 2) {
        echo "nested\n";
    }
}
