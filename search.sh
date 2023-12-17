echo '{
    "q": "'$1'",
    "hybrid": {
        "semanticRatio": 0.5,
        "embedder": "default"
    },
    "showRankingScoreDetails": true
}' | xh POST 'http://localhost:7700/indexes/convs/search' -b | jq '.hits'
