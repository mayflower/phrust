<?php
// runtime-semantics: category=enums expect=fail
enum CannotConstruct {
    case A;
}

new CannotConstruct();
