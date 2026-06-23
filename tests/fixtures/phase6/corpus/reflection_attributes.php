<?php
// phase6-diff: id=PHASE6_CORPUS_REFLECTION_ATTRIBUTES area=corpus expect=pass
// purpose: Reflection-based attribute discovery used by routing and DI metadata scanners.
// reference-output:
// class=Phase6CorpusRoute
// attr=Phase6CorpusRoute
// method=index
#[Attribute(Attribute::TARGET_CLASS | Attribute::TARGET_METHOD)]
class Phase6CorpusRoute
{
    public function __construct(public string $path)
    {
    }
}

#[Phase6CorpusRoute('/home')]
class Phase6CorpusController
{
    #[Phase6CorpusRoute('/index')]
    public function index()
    {
    }
}

$class = new ReflectionClass('Phase6CorpusController');
echo 'class=', $class->getAttributes()[0]->getName(), "\n";
echo 'attr=', (new ReflectionClass('Phase6CorpusRoute'))->getName(), "\n";
echo 'method=', $class->getMethods()[0]->getName(), "\n";
