// Special target: https://github.com/docker/metadata-action#bake-definition
target "docker-metadata-action" {}

group "release" {
  targets = ["build", "build-nochef"]
}

target "build" {
  inherits = ["docker-metadata-action"]
  context = "./"
  dockerfile = "Dockerfile"
  platforms = [
    "linux/amd64",
    #"linux/arm64",
  ]
}

target "build-nochef" {
  inherits = ["docker-metadata-action"]
  context = "./"
  dockerfile = "Dockerfile.nochef"
  platforms = [
    "linux/arm64",
  ]
}
