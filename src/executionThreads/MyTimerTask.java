package executionThreads;

import guiElements.CloseButton;

public class MyTimerTask implements Runnable {
	private CloseButton b;
	public MyTimerTask(CloseButton b) {
		this.b = b;
	}
	
	public void run()
	{
		b.setEnabled(true);
		System.out.println("Button has been enabled");
	}
}
