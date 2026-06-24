<?php
// runtime-semantics: category=enums expect=pass
enum Code: int {
    case Ok = 200;
    case Missing = 404;
}

echo Code::from(404)->name . ":" . Code::Ok->value . "\n";
