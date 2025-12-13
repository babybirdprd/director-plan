from playwright.sync_api import Page, expect, sync_playwright
import os
import time

def test_ticket_diff_verification(page: Page):
    print("Navigating to homepage...")
    page.goto("http://localhost:3000")

    print("Waiting for T-FAIL...")
    # Wait for the sidebar item to be visible.
    expect(page.get_by_text("T-FAIL")).to_be_visible(timeout=10000)

    output_list = "/home/jules/verification/verification_list.png"
    print(f"Taking list screenshot to {output_list}...")
    page.screenshot(path=output_list, full_page=True)

    print("Clicking T-FAIL...")
    # Click the sidebar item.
    page.get_by_text("T-FAIL").click()

    print("Waiting for ticket details...")
    # We expect the H2 header in the main view
    expect(page.get_by_role("heading", name="Visual Regression Failure Test", level=2)).to_be_visible()

    print("Waiting for images...")
    time.sleep(3) # Give explicit time for images to load

    output_detail = "/home/jules/verification/verification_detail.png"
    print(f"Taking detail screenshot to {output_detail}...")
    page.screenshot(path=output_detail, full_page=True)

if __name__ == "__main__":
    os.makedirs("/home/jules/verification", exist_ok=True)
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        page = browser.new_page()
        page.set_viewport_size({"width": 1600, "height": 900})
        try:
            test_ticket_diff_verification(page)
            print("Verification successful.")
        except Exception as e:
            print(f"Error: {e}")
            try:
                page.screenshot(path="/home/jules/verification/error.png")
            except:
                pass
            raise e
        finally:
            browser.close()
