local_resource('compile', 'just compile')
docker_build('cow-on-board/engula-operator', '.', dockerfile='Dockerfile')
k8s_yaml('yaml/deployment.yaml')
k8s_resource('engula-operator', port_forwards=8080)
