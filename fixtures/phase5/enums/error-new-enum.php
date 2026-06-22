<?php
// phase5-runtime: category=enums expect=fail
enum CannotConstruct {
    case A;
}

new CannotConstruct();
