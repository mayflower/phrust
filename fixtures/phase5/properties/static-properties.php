<?php
class StaticProperties {
    public static int $count;
    public static $name = 'shared';
}
StaticProperties::$count = 2;
echo StaticProperties::$count, '|', StaticProperties::$name, "\n";
