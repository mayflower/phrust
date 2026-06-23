<?php
namespace Phase6\Corpus;

class ContainerService
{
    public function label($name)
    {
        return 'service:' . \strtoupper($name);
    }
}
