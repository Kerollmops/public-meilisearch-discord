echo '{ "vectorStore": true, "scoreDetails": true }' | xh PATCH 'http://localhost:7700/experimental-features'

echo '{
    "embedders": {
        "default": {
            "source": {
                "openAi": {}
            },
            "documentTemplate": {
                "template": "The conversation is titled {{doc.title}} and here is a summary of it: {{doc.body}}"
            }
        }
    }
}' | xh PATCH 'http://localhost:7700/indexes/convs/settings'

cat summarizes.jsonl | xh POST 'http://localhost:7700/indexes/convs/documents' 'content-type:application/x-ndjson'
