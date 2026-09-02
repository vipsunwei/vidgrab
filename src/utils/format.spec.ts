import { describe, it, expect } from 'vitest';
import { detectPlatform } from './format';

describe('detectPlatform', () => {
  it('识别哔哩哔哩', () => {
    expect(detectPlatform('https://www.bilibili.com/video/BV1xx')).toBe('哔哩哔哩');
    expect(detectPlatform('https://b23.tv/abc')).toBe('哔哩哔哩');
  });

  it('识别 YouTube', () => {
    expect(detectPlatform('https://www.youtube.com/watch?v=abc')).toBe('YouTube');
    expect(detectPlatform('https://youtu.be/xyz')).toBe('YouTube');
  });

  it('识别 Twitter / X（含子域与大小写）', () => {
    expect(detectPlatform('https://twitter.com/foo/status/1')).toBe('Twitter / X');
    expect(detectPlatform('https://x.com/foo')).toBe('Twitter / X');
    expect(detectPlatform('https://mobile.x.com/foo')).toBe('Twitter / X');
    expect(detectPlatform('HTTPS://WWW.X.COM/foo')).toBe('Twitter / X');
  });

  it('x.com 不应误匹配其它域名', () => {
    // netflix.com 末尾标签为 [netflix, com]，不能因包含子串 "x.com" 误判
    expect(detectPlatform('https://www.netflix.com/watch/1')).toBe('未知平台');
    expect(detectPlatform('https://xvideo.com/foo')).toBe('未知平台');
    expect(detectPlatform('https://example.com/x.com')).toBe('未知平台');
  });

  it('未知平台返回兜底值', () => {
    expect(detectPlatform('https://example.com/video')).toBe('未知平台');
    expect(detectPlatform('')).toBe('未知平台');
  });
});
