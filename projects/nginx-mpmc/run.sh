# save pwd variable
# append pwd to nginx.conf prefix
# run this command with the new nginx.conf path
(cd app-1 && cargo leptos serve)  & \
(cd app-2 && cargo leptos serve) & \
(cd shared-server-1 && cargo run) & \
(cd shared-server-2 && cargo run) & \
( current_dir=$(pwd) && \
docker run --rm -v "$current_dir"/nginx.conf:/etc/nginx/nginx.conf:ro -p 80:80 nginx)
