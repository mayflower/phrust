<?php
class MethodReturnFixture {
    function answer() {
        return 42;
    }
}

$returner = new MethodReturnFixture();
echo $returner->answer(), "\n";
