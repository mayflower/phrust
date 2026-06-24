<?php
// stdlib-diff: id=STDLIB_CORPUS_REFLECTION_ATTRIBUTES area=corpus expect=pass
// purpose: Reflection-based attribute discovery used by routing and DI metadata scanners.
// reference-output:
// class=StdlibCorpusRoute
// attr=StdlibCorpusRoute
// method=index
#[Attribute(Attribute::TARGET_CLASS | Attribute::TARGET_METHOD)]
class StdlibCorpusRoute
{
    public function __construct(public string $path)
    {
    }
}

#[StdlibCorpusRoute('/home')]
class StdlibCorpusController
{
    #[StdlibCorpusRoute('/index')]
    public function index()
    {
    }
}

$class = new ReflectionClass('StdlibCorpusController');
echo 'class=', $class->getAttributes()[0]->getName(), "\n";
echo 'attr=', (new ReflectionClass('StdlibCorpusRoute'))->getName(), "\n";
echo 'method=', $class->getMethods()[0]->getName(), "\n";
