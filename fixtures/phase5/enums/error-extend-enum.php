<?php
// phase5-runtime: category=enums expect=fail
enum EnumParent {
    case A;
}

class EnumChild extends EnumParent {
}

new EnumChild();
