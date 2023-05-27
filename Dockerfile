FROM nginx
ARG PORT
COPY nginx.conf.template /
RUN sed -e "s/@PORT@/${PORT}/g" nginx.conf.template >/etc/nginx/conf.d/default.conf
COPY nginx.conf /etc/nginx
RUN echo -e "* hard nproc 1048576\n* soft nproc 1048576\n" >/etc/security/limits.conf
