<?php
// phase5-runtime: expect=fail
class HiddenStaticProperty {
    private static int $value;
}
HiddenStaticProperty::$value = 1;
