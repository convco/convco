#-*-mode:hcl;indent-tabs-mode:nil;tab-width:2;coding:utf-8-*-
# vi: ft=hcl tabstop=2 shiftwidth=2 softtabstop=2 expandtab:

# [ NOTE ] => clean up buildx builders
# docker buildx ls | awk '$2 ~ /^docker(-container)*$/{print $1}' | xargs -r -I {} docker buildx rm {}
# [ NOTE ] create a builder for this file
# docker buildx create --use --name "convco" --driver docker-container
# [ NOTE ] run build without pushing to dockerhub
# LOCAL=true docker buildx bake --builder convco

variable "LOCAL" {default=false}
variable "ARM64" {default=true}
variable "AMD64" {default=true}
variable "TAG" {default=""}
variable "IMAGE_NAME" {default="convco/convco"}
group "default" {
  targets = [
    "convco"
  ]
}
# LOCAL=true docker buildx bake --builder convco convco
# LOCAL=true ARM64=false AMD64=true docker buildx bake --builder convco convco
# LOCAL=true ARM64=true AMD64=false docker buildx bake --builder convco convco
target "convco" {
  context="."
  dockerfile = "Dockerfile"
  tags = [
    "${IMAGE_NAME}:latest",
    notequal("",TAG) ? "${IMAGE_NAME}:${TAG}": "",
  ]
  platforms = [
    equal(AMD64,true) ?"linux/amd64":"",
    equal(ARM64,true) ?"linux/arm64":"",
  ]
  cache-from = ["type=registry,ref=${IMAGE_NAME}:cache"]
  cache-to   = [equal(LOCAL,false) ? "type=registry,mode=max,ref=${IMAGE_NAME}:cache" : ""]
  output     = [equal(LOCAL,true) ? "type=docker" : "type=registry"]
}
