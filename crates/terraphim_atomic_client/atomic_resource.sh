#!/bin/bash

# Usage: ./atomic_resource.sh <command> [args...]
# Commands:
#   create <shortname> <name> <description> <class>
#   read <resource_url>
#   update <resource_url> <property> <value>
#   delete <resource_url>
#   search <query>

# Default values
SERVER_URL=${ATOMIC_SERVER_URL:-"http://localhost:9883"}
TOKEN="eyJwcml2YXRlS2V5IjoiR29xTEdDeFVkaE80cEJWSkVhclUrb2UrUDdPNVJFS3VzRnE3UjAzeDdPZz0iLCJwdWJsaWNLZXkiOiJ6WjJTMEhUZzE3UE01WUxRSkhZa3lGeEcvQmFMMHVYU2h3N0thQjFDSk9nPSIsInN1YmplY3QiOiJodHRwOi8vbG9jYWxob3N0Ojk4ODMvYWdlbnRzL3paMlMwSFRnMTdQTTVZTFFKSFlreUZ4Ry9CYUwwdVhTaHc3S2FCMUNKT2c9IiwiY2xpZW50Ijp7fX0="

# Function to create a resource
create_resource() {
    local shortname=$1
    local name=$2
    local description=$3
    local class=$4

    echo "Creating ${class}: ${name}"

    # Generate the commit JSON
    cd test_signature
    export ATOMIC_SERVER_SECRET="$TOKEN"
    cargo run create "$shortname" "$description" "$class" "$name" > ../commit.json
    cd ..

    # Send the commit to the server
    echo "Sending commit to server..."
    curl -s -H "Content-Type: application/json" -d @commit.json ${SERVER_URL}/commit | jq .

    # Check if the resource was created
    echo "Checking if resource was created..."
    curl -s -H "Accept: application/json" ${SERVER_URL}/${shortname} | jq .
}

# Function to read a resource
read_resource() {
    local resource_url=$1
    echo "Reading resource: ${resource_url}"
    curl -s -H "Accept: application/json" ${resource_url} | jq .
}

# Function to update a resource
update_resource() {
    local resource_url=$1
    local property=$2
    local value=$3

    echo "Updating resource: ${resource_url}"
    echo "Setting ${property} to ${value}"

    # Generate the commit JSON
    cd test_signature
    export ATOMIC_SERVER_SECRET="$TOKEN"
    cargo run update "$resource_url" "$property" "$value" > ../commit_update.json
    cd ..

    # Send the commit to the server
    curl -s -H "Content-Type: application/json" -d @commit_update.json ${SERVER_URL}/commit | jq .

    # Check if the resource was updated
    curl -s -H "Accept: application/json" ${resource_url} | jq .
}

# Function to delete a resource
delete_resource() {
    local resource_url=$1

    echo "Deleting resource: ${resource_url}"

    # Generate the commit JSON
    cd test_signature
    export ATOMIC_SERVER_SECRET="$TOKEN"
    cargo run delete "$resource_url" > ../commit_delete.json
    cd ..

    # Send the commit to the server
    curl -s -H "Content-Type: application/json" -d @commit_delete.json ${SERVER_URL}/commit | jq .

    # Check if the resource was deleted
    echo "Checking if resource was deleted..."
    curl -s -H "Accept: application/json" ${resource_url} | jq .
}

# Function to search for resources
search_resources() {
    local query=$1
    echo "Searching for: ${query}"
    curl -s -H "Accept: application/json" -G --data-urlencode "q=${query}" ${SERVER_URL}/search | jq .
}

# Main command handler
case "$1" in
    create)
        if [ $# -lt 5 ]; then
            echo "Usage: $0 create <shortname> <name> <description> <class>"
            exit 1
        fi
        create_resource "$2" "$3" "$4" "$5"
        ;;
    read)
        if [ $# -lt 2 ]; then
            echo "Usage: $0 read <resource_url>"
            exit 1
        fi
        read_resource "$2"
        ;;
    update)
        if [ $# -lt 4 ]; then
            echo "Usage: $0 update <resource_url> <property> <value>"
            exit 1
        fi
        update_resource "$2" "$3" "$4"
        ;;
    delete)
        if [ $# -lt 2 ]; then
            echo "Usage: $0 delete <resource_url>"
            exit 1
        fi
        delete_resource "$2"
        ;;
    search)
        if [ $# -lt 2 ]; then
            echo "Usage: $0 search <query>"
            exit 1
        fi
        search_resources "$2"
        ;;
    *)
        echo "Usage: $0 <command> [args...]"
        echo "Commands:"
        echo "  create <shortname> <name> <description> <class>"
        echo "  read <resource_url>"
        echo "  update <resource_url> <property> <value>"
        echo "  delete <resource_url>"
        echo "  search <query>"
        exit 1
        ;;
esac
