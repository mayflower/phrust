<?php
class Prompt27Returner {
    function answer() {
        return 42;
    }
}

$returner = new Prompt27Returner();
echo $returner->answer(), "\n";
