<?php
// phase5-runtime: category=enums expect=pass
enum Status {
    case Draft;
    case Published;
}

foreach (Status::cases() as $case) {
    echo $case->name . "\n";
}
