<?php

class NoParentFixture
{
    public function ping(): void
    {
        parent::ping();
    }
}
