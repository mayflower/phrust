<?php
// runtime-semantics: expect=fail
class HiddenStaticProperty {
    private static int $value;
}
HiddenStaticProperty::$value = 1;
