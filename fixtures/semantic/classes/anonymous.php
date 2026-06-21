<?php

$value = new class extends BaseThing implements RunnableThing {
    use SharedBehavior;

    public function run(): void
    {
    }
};
