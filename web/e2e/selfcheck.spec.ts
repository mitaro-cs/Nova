import { expect, test } from './util-test';
import { expectClean, url } from './util';

// Сама проверка вёрстки должна ловить ошибки, иначе её зелёный цвет ничего не значит.
test('самопроверка: наложение ловится', async ({ page }) => {
  // Своя проба на странице: заголовок и абзац под ним (страницы меняют вид, проба - нет). Абзац заезжает на заголовок.
  await page.goto(url('/my'));
  await page.waitForSelector('main');
  await page.evaluate(() => {
    const box = document.createElement('section');
    box.id = 'probe';
    box.innerHTML = '<h2 style="font-size:32px;margin:0 0 -20px">Проба заголовка</h2><p style="margin:0">Текст под заголовком</p>';
    document.querySelector('main')!.prepend(box);
  });
  await expect(expectClean(page, 'проба')).rejects.toThrow(/пересекаются/);
  await page.evaluate(() => {
    document.getElementById('probe')!.remove();
    const d = document.createElement('div');
    d.style.cssText = 'width:2000px;height:10px';
    d.textContent = 'x';
    document.querySelector('main')!.append(d);
  });
  await expect(expectClean(page, 'проба')).rejects.toThrow(/шире экрана/);
});
