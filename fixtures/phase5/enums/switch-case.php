<?php
// phase5-runtime: category=enums expect=pass
enum Mode {
    case Read;
    case Write;
}

$mode = Mode::Write;
switch ($mode) {
    case Mode::Read:
        echo "read\n";
        break;
    case Mode::Write:
        echo "write\n";
        break;
}
