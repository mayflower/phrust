<?php
interface Runnable {
    public function run(): string;
}

class Job implements Runnable {
    public function run(): string { return "run"; }
}

echo (new Job())->run();
