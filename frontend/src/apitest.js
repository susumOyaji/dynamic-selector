document.addEventListener('DOMContentLoaded', () => {
    const quoteForm = document.getElementById('quote-form');
    const verifyForm = document.getElementById('verify-form');
    const parserForm = document.getElementById('parser-form');
    const responseEl = document.getElementById('api-response').querySelector('code');

    // When deployed, this might need to be an absolute URL to your worker.
    const API_BASE_URL = 'http://127.0.0.1:8787';

    const displayResult = (data) => {
        responseEl.textContent = JSON.stringify(data, null, 2);
    };

    const displayLoading = () => {
        responseEl.textContent = 'Loading...';
    };

    const displayError = (err) => {
        responseEl.textContent = `Error: ${err.message}\n\nCheck the browser console for more details. Make sure the worker is running.`;
        console.error(err);
    };

    // Handler for /api/quote
    quoteForm.addEventListener('submit', async (e) => {
        e.preventDefault();
        displayLoading();
        const codes = document.getElementById('quote-codes').value;
        const url = new URL(`${API_BASE_URL}/api/quote`);
        url.searchParams.append('codes', codes);

        try {
            const res = await fetch(url);
            if (!res.ok) throw new Error(`HTTP error! status: ${res.status}`);
            const data = await res.json();
            displayResult(data);
        } catch (err) {
            displayError(err);
        }
    });

    // Handler for /api/verify-selectors
    verifyForm.addEventListener('submit', async (e) => {
        e.preventDefault();
        displayLoading();
        const codeType = document.getElementById('verify-code-type').value;
        const code = document.getElementById('verify-code').value;
        const url = new URL(`${API_BASE_URL}/api/verify-selectors`);
        url.searchParams.append('code_type', codeType);
        url.searchParams.append('code', code);

        try {
            const res = await fetch(url);
            if (!res.ok) throw new Error(`HTTP error! status: ${res.status}`);
            const data = await res.json();
            displayResult(data);
        } catch (err) {
            displayError(err);
        }
    });

    // Handler for /api/test-parser
    parserForm.addEventListener('submit', async (e) => {
        e.preventDefault();
        displayLoading();
        const htmlContent = document.getElementById('parser-html').value;
        const code = document.getElementById('parser-code').value;
        const selectorsStr = document.getElementById('parser-selectors').value;

        let selectors;
        try {
            if (selectorsStr.trim() === '') {
                selectors = {};
            } else {
                selectors = JSON.parse(selectorsStr);
            }
        } catch (err) {
            displayError(new Error(`Invalid JSON in Selectors field: ${err.message}`));
            return;
        }
        
        if (!htmlContent) {
            displayError(new Error('HTML Content is required for the parser test.'));
            return;
        }

        const body = {
            html_content: htmlContent,
            code: code,
            selectors: selectors,
        };

        try {
            const res = await fetch(`${API_BASE_URL}/api/test-parser`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify(body),
            });
            if (!res.ok) throw new Error(`HTTP error! status: ${res.status}`);
            const data = await res.json();
            displayResult(data);
        } catch (err) {
            displayError(err);
        }
    });
});
