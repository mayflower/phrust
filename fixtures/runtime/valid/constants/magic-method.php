<?php
class Prompt24MagicMethod
{
    public function show() {
        echo __CLASS__, "|", __METHOD__, "\n";
    }
}

(new Prompt24MagicMethod())->show();
