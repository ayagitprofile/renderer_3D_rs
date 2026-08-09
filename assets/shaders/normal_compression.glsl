#ifndef NORMAL_COMPRESSION_INCLUDED
#define NORMAL_COMPRESSION_INCLUDED
vec2 sign_not_zero(vec2 v)
{
    return vec2(
        v.x < 0.0 ? -1.0 : 1.0,
        v.y < 0.0 ? -1.0 : 1.0
    );
}

vec2 normal_oct_encoding(vec3 n)
{
    n /= (abs(n.x) + abs(n.y) + abs(n.z));

    vec2 p = n.xy;

    if (n.z < 0.0)
    {
        p = (1.0 - abs(p.yx)) * sign_not_zero(p);
    }

    return p * 0.5 + 0.5;
}

vec3 normal_oct_decode(vec2 e)
{
    e = e * 2.0 - 1.0;

    vec3 n = vec3(e.x, e.y, 1.0 - abs(e.x) - abs(e.y));

    if (n.z < 0.0)
    {
        n.xy = (1.0 - abs(n.yx)) * sign_not_zero(n.xy);
    }

    return normalize(n);
}
#endif