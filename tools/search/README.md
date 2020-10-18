```
   ____          _      ____                      _     
  / ___|___   __| | ___/ ___|  ___  __ _ _ __ ___| |__  
 | |   / _ \ / _` |/ _ \___ \ / _ \/ _` | '__/ __| '_ \ 
 | |__| (_) | (_| |  __/___) |  __/ (_| | | | (__| | | |
  \____\___/ \__,_|\___|____/ \___|\__,_|_|  \___|_| |_|
```

  Author:   Colin Merkel (colin.merkel@gmail.com)
  Github:   https://github.com/colin353/universe/tree/master/tools/search

## Overview

 - Scratch written trigram-based search engine for code
 - Focused on ranking search results by relevance
 - Ranking partly based on `PageRank`

## Usage

You can run the combined indexer/server using a single docker command. First
`cd` to a directory containing your code, then run:

```
docker run -p 9898:9898 -v $PWD:/code colinmerkel/code_search
```

First, the indexer will run and build an index of your code. For a codebase
with 2M lines of code it takes roughly a minute to index all the code. After that, it'll 
start serving at http://localhost:9898/

If you want to change the port, you can do that like this:

```
PORT=7878; docker run -p $PORT:$PORT -v $PWD:/code colinmerkel/code_search --web_port=$PORT
```
