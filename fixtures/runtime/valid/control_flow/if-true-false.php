<?php
// runtime-fixture: kind=valid expected_stdout="tf\n"
if (true) {
    echo "t";
}
if (false) {
    echo "bad";
} else {
    echo "f";
}
echo "\n";
