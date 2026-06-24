<?php
class StaticPropertyFixture {
    static $value;
}

StaticPropertyFixture::$value = 1;
echo StaticPropertyFixture::$value, "\n";
