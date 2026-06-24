<?php
class StaticUtilFixture {
    static function name() {
        return "static-ok";
    }
}

echo StaticUtilFixture::name(), "\n";
