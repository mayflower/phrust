<?php
class MagicMethodFixture
{
    public function show() {
        echo __CLASS__, "|", __METHOD__, "\n";
    }
}

(new MagicMethodFixture())->show();
